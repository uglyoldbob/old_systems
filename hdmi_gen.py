from migen import *
from migen.genlib.cdc import MultiReg
from litex.gen import *
from litex.soc.interconnect.csr import *
from litex.soc.interconnect import stream
from litex.soc.cores.video import video_data_layout
from litex.soc.cores.video import video_timing_layout
from litex.soc.cores.video import hbits, vbits, video_timings

video_extra_data_layout = [
    ("start", 1),
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

        self._hres        = CSRStorage(hbits, vt["h_active"],
            description="Horizontal active resolution.")
        self._hsync_start = CSRStorage(hbits, vt["h_active"] + vt["h_sync_offset"],
            description="Horizontal sync start.")
        self._hsync_end   = CSRStorage(hbits, vt["h_active"] + vt["h_sync_offset"] + vt["h_sync_width"],
            description="Horizontal sync end.")
        self._hscan       = CSRStorage(hbits, vt["h_active"] + vt["h_blanking"] - 1,
            description="Horizontal scan period.")

        self._vres        = CSRStorage(vbits, vt["v_active"],
            description="Vertical active resolution.")
        self._vsync_start = CSRStorage(vbits, vt["v_active"] + vt["v_sync_offset"],
            description="Vertical sync start.")
        self._vsync_end   = CSRStorage(vbits, vt["v_active"] + vt["v_sync_offset"] + vt["v_sync_width"],
            description="Vertical sync end.")
        self._vscan       = CSRStorage(vbits, vt["v_active"] + vt["v_blanking"] - 1,
            description="Vertical scan period.")
        self._delay_lines = CSRStorage(vbits, 3, description="Number of vertical lines of signal for delay")

        # Video Timing Source
        self.source = source = stream.Endpoint(video_timing_layout)

        # # #

        # Resynchronize Enable to Video clock domain.
        self.enable = enable = Signal()
        self.specials += MultiReg(self._enable.storage, enable)

        # Resynchronize Horizontal Timings to Video clock domain.
        self.hres        = hres        = Signal(hbits)
        self.hsync_start = hsync_start = Signal(hbits)
        self.hsync_end   = hsync_end   = Signal(hbits)
        self.hscan       = hscan       = Signal(hbits)
        self.delay_lines = delay_lines = Signal(vbits)
        self.specials += MultiReg(self._hres.storage,        hres)
        self.specials += MultiReg(self._hsync_start.storage, hsync_start)
        self.specials += MultiReg(self._hsync_end.storage,   hsync_end)
        self.specials += MultiReg(self._hscan.storage,       hscan)
        self.specials += MultiReg(self._delay_lines.storage, delay_lines)

        # Resynchronize Vertical Timings to Video clock domain.
        self.vres        = vres        = Signal(vbits)
        self.vsync_start = vsync_start = Signal(vbits)
        self.vsync_end   = vsync_end   = Signal(vbits)
        self.vscan       = vscan       = Signal(vbits)
        self.specials += MultiReg(self._vres.storage,        vres)
        self.specials += MultiReg(self._vsync_start.storage, vsync_start)
        self.specials += MultiReg(self._vsync_end.storage,   vsync_end)
        self.specials += MultiReg(self._vscan.storage,       vscan)

        pre_vstart = Signal(vbits)
        self.comb += pre_vstart.eq(vscan - delay_lines)
        pre_active = Signal()
        self.comb += pre_active.eq(source.vcount >= pre_vstart)

        # Generate timings.
        pre_active = Signal()
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
                If(source.hcount == 0,           NextValue(hactive,       1)), # Start of HActive.
                If(source.hcount == hres,        NextValue(hactive,       0)), # End of HActive.
                If(source.hcount == hsync_start, NextValue(source.hsync,  1)),
                If(source.hcount == hsync_end,   NextValue(source.hsync,  0)), # End of HSync.
                If(source.hcount == hscan,       NextValue(source.hcount, 0)), # End of HScan.

                If(source.hcount == hsync_start,
                    # Increment VCount.
                    NextValue(source.vcount, source.vcount + 1),
                    # Generate VActive / VSync.
                    If(source.vcount == 0,           NextValue(vactive,       1)), # Start of VActive.
                    If(source.vcount == vres,        NextValue(vactive,       0)), # End of VActive.
                    If(source.vcount == vsync_start, NextValue(source.vsync,  1)),
                    If(source.vcount == vsync_end,   NextValue(source.vsync,  0)), # End of VSync.
                    If(source.vcount == vscan,       NextValue(source.vcount, 0))  # End of VScan.
                )
            )
        )
        self.extra_source = stream.Endpoint(video_extra_data_layout)
        self.sync += self.extra_source.start.eq(pre_active)

class HdmiGenerator(LiteXModule):
    def __init__(self, default_video_timings="800x600@60Hz"):
        self.dummy = default_video_timings
        self.extra_sink = stream.Endpoint(video_extra_data_layout)
        self.vtg_sink = stream.Endpoint(video_timing_layout)
        self.source   = stream.Endpoint(video_data_layout)
        self.data = Signal(32)
        self.random = Signal(32)
        active = Signal()
        self.specials += Instance("lfsr32_hpf",
            i_clock     = ClockSignal("hdmi"),
            i_hp_enable = 1,
            o_dout    = self.random,
        )
        self.comb += [
            If((self.vtg_sink.hcount > 255) & (self.vtg_sink.hcount < 1023),
                active.eq(1)
            ).Else(
                active.eq(0)
            ),
        ]

        enable = Signal()
        enable_line = Signal()
        counter = Signal(max=3)
        active_r = Signal()
        line_start = Signal()
        line_counter = Signal(max=3)
        self.sync.hdmi += active_r.eq(active)
        self.comb += line_start.eq(active & ~active_r)
        self.comb += enable.eq(line_counter == 0)

        self.sync.hdmi += [
            If(self.vtg_sink.vsync,
                line_counter.eq(0),
            ).Elif(line_start,
                If(line_counter == 2,
                    line_counter.eq(0),
                ).Else(
                    line_counter.eq(line_counter + 1),
                ),
            ),
        ]

        self.comb += enable_line.eq(active & (line_counter == 0))
        self.comb += enable.eq(active & (counter == 0))

        self.sync.hdmi += [
            If(active,
                If(counter == 2,
                    counter.eq(0),
                ).Else(
                    counter.eq(counter + 1),
                ),
            ).Else(
                counter.eq(0),
            ),
        ]
        self.sync.hdmi += [
            If(active, 
               If(enable & enable_line,
                    self.data.eq(self.random)
                ).Else(
                    self.data.eq(0)
                ),
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
