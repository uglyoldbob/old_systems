from migen import *
from migen.genlib.cdc import MultiReg
from litex.gen import *
from litex.soc.interconnect.csr import *
from litex.soc.interconnect import stream
from litex.soc.cores.video import video_data_layout
from litex.soc.cores.video import video_timing_layout
from litex.soc.cores.video import hbits, vbits, video_timings

video_extra_data_layout = [
    ("row_process", 1),
    ("col_process", 1),
    ("process", 1),
]

class VideoTimingGenerator(LiteXModule):
    def __init__(self, default_video_timings="800x600@60Hz"):
        # Check / Get Video Timings (can be str or dict)
        if isinstance(default_video_timings, str):
            try:
                self.video_timings = vt = video_timings[default_video_timings]
            except KeyError:
                msg = [f"Video Timings {default_video_timings} not supported, availables:"]
                for video_timing in video_timings.keys():
                    msg.append(f" - {video_timing} / {video_timings[video_timing]['pix_clk']/1e6:3.2f}MHz.")
                raise ValueError("\n".join(msg))
        else:
            self.video_timings = vt = default_video_timings

        # MMAP Control/Status Registers.
        self._enable      = CSRStorage(reset=1, description="Video Timing Generator enable.")

        self._hres        = CSRStorage(hbits, 6 + vt["h_active"],
            description="Horizontal active resolution.")
        self._hsync_start = CSRStorage(hbits, 6 + vt["h_active"] + vt["h_sync_offset"],
            description="Horizontal sync start.")
        self._hsync_end   = CSRStorage(hbits, 6 + vt["h_active"] + vt["h_sync_offset"] + vt["h_sync_width"],
            description="Horizontal sync end.")
        self._hscan       = CSRStorage(hbits, vt["h_active"] + vt["h_blanking"] - 1,
            description="Horizontal scan period.")

        self._vres        = CSRStorage(vbits, 3 + vt["v_active"],
            description="Vertical active resolution.")
        self._vsync_start = CSRStorage(vbits, 3 + vt["v_active"] + vt["v_sync_offset"],
            description="Vertical sync start.")
        self._vsync_end   = CSRStorage(vbits, 3 + vt["v_active"] + vt["v_sync_offset"] + vt["v_sync_width"],
            description="Vertical sync end.")
        self._vscan       = CSRStorage(vbits, vt["v_active"] + vt["v_blanking"] - 1,
            description="Vertical scan period.")
        
        self._pre_h_start = CSRStorage(hbits, 256, description="Start of processing for smaller image")
        self._pre_h_stop = CSRStorage(hbits, 3 + 256 + 768, description="End of processing for smaller image")

        # Video Timing Source
        self.source = source = stream.Endpoint(video_timing_layout)
        self.esource = esource = stream.Endpoint(video_extra_data_layout)

        # # #

        # Resynchronize Enable to Video clock domain.
        self.enable = enable = Signal()
        self.specials += MultiReg(self._enable.storage, enable)

        # Resynchronize Horizontal Timings to Video clock domain.
        self.hres        = hres        = Signal(hbits)
        self.hsync_start = hsync_start = Signal(hbits)
        self.hsync_end   = hsync_end   = Signal(hbits)
        self.hscan       = hscan       = Signal(hbits)
        self.pre_h_start = pre_h_start = Signal(hbits)
        self.pre_h_stop = pre_h_stop = Signal(hbits)
        self.specials += MultiReg(self._hres.storage,        hres)
        self.specials += MultiReg(self._hsync_start.storage, hsync_start)
        self.specials += MultiReg(self._hsync_end.storage,   hsync_end)
        self.specials += MultiReg(self._hscan.storage,       hscan)
        self.specials += MultiReg(self._pre_h_start.storage, pre_h_start)
        self.specials += MultiReg(self._pre_h_stop.storage,  pre_h_stop)

        # Resynchronize Vertical Timings to Video clock domain.
        self.vres        = vres        = Signal(vbits)
        self.vsync_start = vsync_start = Signal(vbits)
        self.vsync_end   = vsync_end   = Signal(vbits)
        self.vscan       = vscan       = Signal(vbits)
        self.specials += MultiReg(self._vres.storage,        vres)
        self.specials += MultiReg(self._vsync_start.storage, vsync_start)
        self.specials += MultiReg(self._vsync_end.storage,   vsync_end)
        self.specials += MultiReg(self._vscan.storage,       vscan)

        # Generate timings.
        phactive = Signal()
        pvactive = Signal()
        hactive = Signal()
        vactive = Signal()
        fsm = FSM(reset_state="IDLE")
        fsm = ResetInserter()(fsm)
        self.fsm = fsm
        self.comb += fsm.reset.eq(~enable)
        fsm.act("IDLE",
            NextValue(hactive, 0),
            NextValue(vactive, 0),
            NextValue(source.hres, hres),
            NextValue(source.vres, vres),
            NextValue(source.hcount,  0),
            NextValue(source.vcount,  0),
            NextState("RUN")
        )
        self.comb += source.de.eq(hactive & vactive) # DE when both HActive and VActive.
        self.sync += source.first.eq((source.hcount ==     0) & (source.vcount ==     0)),
        self.sync += source.last.eq( (source.hcount == hscan) & (source.vcount == vscan)),
        fsm.act("RUN",
            source.valid.eq(1),
            If(source.ready,
                # Increment HCount.
                NextValue(source.hcount, source.hcount + 1),
                # Generate HActive / HSync.
                If(source.hcount == pre_h_start,           NextValue(phactive, 1)),
                If(source.hcount == pre_h_stop,        NextValue(phactive,       0)),
                If(source.hcount == 6,           NextValue(hactive,       1)), # Start of HActive.
                If(source.hcount == hres,        NextValue(hactive,       0)), # End of HActive.
                If(source.hcount == hsync_start, NextValue(source.hsync,  1)),
                If(source.hcount == hsync_end,   NextValue(source.hsync,  0)), # End of HSync.
                If(source.hcount == hscan,       NextValue(source.hcount, 0)), # End of HScan.

                If(source.hcount == hsync_start,
                    # Increment VCount.
                    NextValue(source.vcount, source.vcount + 1),
                    # Generate VActive / VSync.
                    If(source.vcount == 0,           NextValue(pvactive,       1)),
                    If(source.vcount == 3,           NextValue(vactive,       1)), # Start of VActive.
                    If(source.vcount == vres,        NextValue(vactive,       0)), # End of VActive.
                    If(source.vcount == vres,        NextValue(pvactive,       0)),
                    If(source.vcount == vsync_start, NextValue(source.vsync,  1)),
                    If(source.vcount == vsync_end,   NextValue(source.vsync,  0)), # End of VSync.
                    If(source.vcount == vscan,       NextValue(source.vcount, 0))  # End of VScan.
                )
            )
        )
        self.sync += [
            self.esource.row_process.eq(phactive),
            self.esource.col_process.eq(pvactive),
        ]
        self.comb += self.esource.process.eq(phactive & pvactive)

class HdmiGenerator(LiteXModule):
    def __init__(self, default_video_timings="800x600@60Hz"):
        self.dummy = default_video_timings
        self.extra_sink = stream.Endpoint(video_extra_data_layout)
        self.vtg_sink = stream.Endpoint(video_timing_layout)
        self.source   = stream.Endpoint(video_data_layout)
        self.data = Signal(32)
        self.random = Signal(32)
        self.specials += Instance("lfsr32_hpf",
            i_clock     = ClockSignal("hdmi"),
            i_hp_enable = 1,
            o_dout    = self.random,
        )

        self.submodules.nes_clock = ClockDomainsRenamer({"sys": "hdmi"})(NESClockScheduler())
        self.comb += self.vtg_sink.connect(self.nes_clock.source)

        self.sync.hdmi += [
            If(self.nes_clock.ppu_enable, 
               self.data.eq(self.random)
            ).Else(
                self.data.eq(255 * 256)
            )
        ]
        self.comb += self.vtg_sink.ready.eq(1)
        self.comb += self.source.r.eq(self.data[0:8])
        self.comb += self.source.g.eq(self.data[8:16])
        self.comb += self.source.b.eq(self.data[16:24])
        self.comb += self.source.hsync.eq(self.vtg_sink.hsync)
        self.comb += self.source.vsync.eq(self.vtg_sink.vsync)
        self.comb += self.source.de.eq(self.vtg_sink.de)

# ---------------------------------------------------------------------
# NES Clock Scheduler
# ---------------------------------------------------------------------

class NESClockScheduler(Module):
    """
    Generates NES PPU/CPU enable pulses synchronized to HDMI raster timing.

    Features:
    - Fully synchronous to HDMI pixel clock
    - No generated clocks (enable pulses only)
    - Spreads 341 NES PPU cycles across 3 HDMI lines
    - Deterministic phase alignment
    - Configurable horizontal start column
    - CPU enable generated every 3 PPU enables
    """

    def __init__(self,
        h_total      = 1650,
        phase_offset = 256
    ):

        # -------------------------------------------------------------
        # Video timing stream input
        # -------------------------------------------------------------

        self.source = source = stream.Endpoint(video_timing_layout)

        # -------------------------------------------------------------
        # Outputs
        # -------------------------------------------------------------

        self.ppu_enable = Signal()
        self.cpu_enable = Signal()

        # Debug/status
        self.nes_dot    = Signal(9)  # 0..340
        self.nes_line   = Signal(9)
        self.third_line = Signal()

        # -------------------------------------------------------------
        # Constants
        # -------------------------------------------------------------

        NES_DOTS_PER_LINE = 341

        # One NES scanline spans 3 HDMI lines
        HDMI_CLKS_PER_NES_LINE = h_total * 3

        # -------------------------------------------------------------
        # Internal state
        # -------------------------------------------------------------

        v_mod3 = Signal(2)

        # DDS/Bresenham accumulator
        accum = Signal(max=HDMI_CLKS_PER_NES_LINE)

        # CPU divider
        cpu_div = Signal(2)

        # NES scanline start pulse
        line_start = Signal()

        # Active rendering region
        active = Signal()

        # -------------------------------------------------------------
        # Combinational logic
        # -------------------------------------------------------------

        self.comb += [

            # Active HDMI region
            active.eq(source.de),

            # Every third HDMI line
            self.third_line.eq(v_mod3 == 0),

            # Start NES scanline at programmable column
            line_start.eq(
                active &
                (source.hcount == phase_offset) &
                (v_mod3 == 0)
            )
        ]

        # -------------------------------------------------------------
        # Main timing engine
        # -------------------------------------------------------------

        self.sync += [

            # default outputs
            self.ppu_enable.eq(0),
            self.cpu_enable.eq(0),

            # ---------------------------------------------------------
            # Track HDMI lines modulo 3
            # ---------------------------------------------------------
            If(source.hcount == 0,

                If(v_mod3 == 2,

                    v_mod3.eq(0),

                    # advance NES scanline counter
                    If(self.nes_line == 261,
                        self.nes_line.eq(0)
                    ).Else(
                        self.nes_line.eq(self.nes_line + 1)
                    )

                ).Else(
                    v_mod3.eq(v_mod3 + 1)
                )
            ),

            # ---------------------------------------------------------
            # Start of NES scanline
            # ---------------------------------------------------------
            If(line_start,

                accum.eq(0),
                self.nes_dot.eq(0)
            ).

            # ---------------------------------------------------------
            # Spread 341 PPU cycles evenly across 3 HDMI lines
            # ---------------------------------------------------------
            Elif(active,

                accum.eq(accum + NES_DOTS_PER_LINE),

                If(accum >= (HDMI_CLKS_PER_NES_LINE - NES_DOTS_PER_LINE),

                    accum.eq(
                        accum
                        + NES_DOTS_PER_LINE
                        - HDMI_CLKS_PER_NES_LINE
                    ),

                    # -------------------------------------------------
                    # Generate PPU enable
                    # -------------------------------------------------
                    self.ppu_enable.eq(1),

                    # NES dot counter
                    If(self.nes_dot == 340,
                        self.nes_dot.eq(0)
                    ).Else(
                        self.nes_dot.eq(self.nes_dot + 1)
                    ),

                    # -------------------------------------------------
                    # Generate CPU enable every 3 PPU cycles
                    # -------------------------------------------------
                    If(cpu_div == 2,

                        cpu_div.eq(0),
                        self.cpu_enable.eq(1)

                    ).Else(
                        cpu_div.eq(cpu_div + 1)
                    )
                )
            )
        ]