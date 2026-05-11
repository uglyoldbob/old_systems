from migen import *
from migen.genlib.cdc import MultiReg
from litex.gen import *
from litex.soc.interconnect.csr import *
from litex.soc.interconnect import stream
from litex.soc.cores.video import video_data_layout
from litex.soc.cores.video import video_timing_layout
from litex.soc.cores.video import hbits, vbits, video_timings

from .nes.hdl.nes import NesSystem, nes_system_inputs, nes_system_outputs

video_extra_data_layout = [
    ("row_process", 1),
    ("col_process", 1),
    ("process", 1),
    ("ppu_enable", 1),
    ("ppu_col", 9),
    ("ppu_row", 9),
    ("ppu_count", 9),
    ("ppu_vcount", 9),
    ("ppu_enable_count", 17),
    ("last_ppu_enable_count", 17),
    ("fast", 1),
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

        self.nes_ppu_end = Signal()

        # MMAP Control/Status Registers.
        self._enable      = CSRStorage(reset=1, description="Video Timing Generator enable.")

        self._hres        = CSRStorage(hbits, 7 + vt["h_active"],
            description="Horizontal active resolution.")
        self._hsync_start = CSRStorage(hbits, 7 + vt["h_active"] + vt["h_sync_offset"],
            description="Horizontal sync start.")
        self._hsync_end   = CSRStorage(hbits, 7 + vt["h_active"] + vt["h_sync_offset"] + vt["h_sync_width"],
            description="Horizontal sync end.")
        self._hscan       = CSRStorage(hbits, vt["h_active"] + vt["h_blanking"] - 1,
            description="Horizontal scan period.")

        self._vres        = CSRStorage(vbits, 3 + vt["v_active"],
            description="Vertical active resolution.")
        self._ppu_vres        = CSRStorage(vbits, vt["v_active"],
            description="Vertical active resolution.")
        self._ppu_vres2        = CSRStorage(vbits, vt["v_active"] + 22,
            description="Vertical active resolution.")
        self._vsync_start = CSRStorage(vbits, 3 + vt["v_active"] + vt["v_sync_offset"],
            description="Vertical sync start.")
        self._vsync_end   = CSRStorage(vbits, 3 + vt["v_active"] + vt["v_sync_offset"] + vt["v_sync_width"],
            description="Vertical sync end.")
        self._vscan       = CSRStorage(vbits, vt["v_active"] + vt["v_blanking"] - 1,
            description="Vertical scan period.")
        
        self._pre_h_start = CSRStorage(hbits, 256, description="Start of processing for smaller image")
        self._pre_h_stop = CSRStorage(hbits, 256 + 341*3, description="End of processing for smaller image")

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
        self.ppu_vres        = ppu_vres        = Signal(vbits)
        self.ppu_vres2        = ppu_vres2        = Signal(vbits)
        self.vsync_start = vsync_start = Signal(vbits)
        self.vsync_end   = vsync_end   = Signal(vbits)
        self.vscan       = vscan       = Signal(vbits)
        self.specials += MultiReg(self._vres.storage,        vres)
        self.specials += MultiReg(self._ppu_vres.storage,        ppu_vres)
        self.specials += MultiReg(self._ppu_vres2.storage,        ppu_vres2)
        self.specials += MultiReg(self._vsync_start.storage, vsync_start)
        self.specials += MultiReg(self._vsync_end.storage,   vsync_end)
        self.specials += MultiReg(self._vscan.storage,       vscan)

        # Generate timings.
        phactive = Signal()
        pvactive = Signal()
        pvactive_fast = Signal()
        hactive = Signal()
        vactive = Signal()
        self.nes_frame_done = nes_frame_done = Signal()
        self.last_cycle_done = last_cycle_done = Signal()
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
            NextValue(esource.ppu_col,  0),
            NextValue(esource.ppu_row,  0),
            NextValue(esource.ppu_count,  0),
            NextValue(esource.ppu_vcount,  0),
            NextValue(esource.ppu_enable_count, 0),
            NextValue(nes_frame_done, 0),
            NextValue(last_cycle_done, 0),
            NextState("RUN")
        )
        self.comb += source.de.eq(hactive & vactive) # DE when both HActive and VActive.
        self.sync += source.first.eq((source.hcount ==     0) & (source.vcount ==     0)),
        self.sync += source.last.eq( (source.hcount == hscan) & (source.vcount == vscan)),
        fsm.act("RUN",
            source.valid.eq(1),
            If(self.nes_ppu_end,        NextValue(nes_frame_done,       1)),
            If(source.ready,
                # Increment HCount.
                NextValue(source.hcount, source.hcount + 1),
                # Generate HActive / HSync.
                If(source.hcount == pre_h_start,           NextValue(phactive, 1)),
                If(source.hcount == pre_h_stop,        NextValue(phactive,       0)),
                If(source.hcount == 7,           NextValue(hactive,       1)), # Start of HActive.
                If(source.hcount == hres,        NextValue(hactive,       0)), # End of HActive.
                If(source.hcount == hsync_start, NextValue(source.hsync,  1)),
                If(source.hcount == hsync_end,   NextValue(source.hsync,  0)), # End of HSync.
                If(source.hcount == hscan,       NextValue(source.hcount, 0)), # End of HScan.
                If(phactive, NextValue(esource.ppu_count, esource.ppu_count + 1))
                    .Else(NextValue(esource.ppu_count, 0)),
                If(esource.ppu_count == 2, 
                    NextValue(esource.ppu_count, 0),
                    NextValue(esource.ppu_col, esource.ppu_col + 1),
                ),
                If(esource.ppu_enable, NextValue(esource.ppu_enable_count, esource.ppu_enable_count + 1)),

                If(source.hcount == hsync_start,
                    # Increment VCount.
                    NextValue(source.vcount, source.vcount + 1),
                    If(pvactive,
                        NextValue(esource.ppu_vcount, esource.ppu_vcount + 1),
                        NextValue(esource.ppu_col, 0),
                    ).Elif(pvactive_fast, NextValue(esource.ppu_vcount, 1))
                     .Else(
                        NextValue(esource.ppu_vcount, 0),
                    ),
                    If(nes_frame_done, NextValue(esource.ppu_vcount, 0)),
                    If(esource.ppu_vcount == 2,
                       NextValue(esource.ppu_vcount, 0),
                       NextValue(esource.ppu_col, 0),
                    ),
                    
                    # Generate VActive / VSync.
                    If(source.vcount == 0,           NextValue(pvactive,       1)),
                    If(source.vcount == 3,           NextValue(vactive,       1)), # Start of VActive.
                    If(source.vcount == vres,        NextValue(vactive,       0)), # End of VActive.
                    If(source.vcount == ppu_vres,        NextValue(pvactive_fast,       1)),
                    If(source.vcount == ppu_vres,        NextValue(pvactive,       0)),
                    If(source.vcount == vsync_start, NextValue(source.vsync,  1)),
                    If(source.vcount == vsync_end,   NextValue(source.vsync,  0)), # End of VSync.
                    If(source.vcount == vscan,
                       NextValue(esource.last_ppu_enable_count, esource.ppu_enable_count),
                        NextValue(source.vcount, 0),
                        NextValue(nes_frame_done, 0),
                        NextValue(esource.ppu_enable_count, 0),
                    ),
                )
            ),
            If(esource.ppu_enable & nes_frame_done, NextValue(last_cycle_done, 1)).Else(NextValue(last_cycle_done, 0)),
            If(last_cycle_done, 
               NextValue(phactive, 0),
               NextValue(esource.ppu_count, 0),
               NextValue(esource.ppu_vcount, 0),
               ),
        )
        self.sync += [
            self.esource.row_process.eq(phactive),
            self.esource.col_process.eq(pvactive),
            self.esource.fast.eq(pvactive_fast),
        ]
        self.comb += self.esource.process.eq(phactive & pvactive)
        self.comb += If((esource.ppu_vcount == 1) & (esource.ppu_count == 1),
            self.esource.ppu_enable.eq(1)).Else(self.esource.ppu_enable.eq(0))

class SevenSegment(LiteXModule):
    """
    4-digit multiplexed seven segment driver.

    Pads definition expected:
        ("seven_segment", 0,
            Subsignal("seg", Pins("...")),  # a,b,c,d,e,f,g
            Subsignal("dp",  Pins("...")),
            Subsignal("en",  Pins("...")),  # digit enables
        )

    Notes:
    - Assumes active-low segments and active-low enables
      (common-anode display, typical on FPGA boards).
    - Set `common_anode=False` for active-high/common-cathode displays.
    """

    def __init__(self, pads, clk_freq=74.25e6, refresh_hz=1000, common_anode=True):
        self.value = Signal(16, reset=0x1234)

        # ---------------------------------------------------------------------
        # Refresh counter
        # ---------------------------------------------------------------------

        refresh_counter_max = int(clk_freq // (refresh_hz * 4))

        refresh_counter = Signal(max=refresh_counter_max)
        digit_index     = Signal(2)

        self.sync += [
            If(refresh_counter == (refresh_counter_max - 1),
                refresh_counter.eq(0),
                digit_index.eq(digit_index + 1)
            ).Else(
                refresh_counter.eq(refresh_counter + 1)
            )
        ]

        # ---------------------------------------------------------------------
        # Extract current nibble
        # ---------------------------------------------------------------------

        current_nibble = Signal(4)

        self.comb += Case(digit_index, {
            0: current_nibble.eq(self.value[0:4]),
            1: current_nibble.eq(self.value[4:8]),
            2: current_nibble.eq(self.value[8:12]),
            3: current_nibble.eq(self.value[12:16]),
        })

        # ---------------------------------------------------------------------
        # Hex digit to 7-segment decode
        #
        # Bit order:
        #   seg[0] = a
        #   seg[1] = b
        #   seg[2] = c
        #   seg[3] = d
        #   seg[4] = e
        #   seg[5] = f
        #   seg[6] = g
        # ---------------------------------------------------------------------

        seg_pattern = Signal(7)

        self.comb += Case(current_nibble, {
            0x0: seg_pattern.eq(0b0111111),
            0x1: seg_pattern.eq(0b0000110),
            0x2: seg_pattern.eq(0b1011011),
            0x3: seg_pattern.eq(0b1001111),
            0x4: seg_pattern.eq(0b1100110),
            0x5: seg_pattern.eq(0b1101101),
            0x6: seg_pattern.eq(0b1111101),
            0x7: seg_pattern.eq(0b0000111),
            0x8: seg_pattern.eq(0b1111111),
            0x9: seg_pattern.eq(0b1101111),
            0xA: seg_pattern.eq(0b1110111),
            0xB: seg_pattern.eq(0b1111100),
            0xC: seg_pattern.eq(0b0111001),
            0xD: seg_pattern.eq(0b1011110),
            0xE: seg_pattern.eq(0b1111001),
            0xF: seg_pattern.eq(0b1110001),
        })

        # ---------------------------------------------------------------------
        # Digit enable decode
        # ---------------------------------------------------------------------

        digit_enable = Signal(4)

        self.comb += Case(digit_index, {
            0: digit_enable.eq(0b0001),
            1: digit_enable.eq(0b0010),
            2: digit_enable.eq(0b0100),
            3: digit_enable.eq(0b1000),
        })

        # ---------------------------------------------------------------------
        # Drive outputs
        # ---------------------------------------------------------------------

        if common_anode:
            # Active low
            self.comb += [
                pads.seg.eq(~seg_pattern),
                pads.dp.eq(1),          # decimal point off
                pads.en.eq(~digit_enable)
            ]
        else:
            # Active high
            self.comb += [
                pads.seg.eq(seg_pattern),
                pads.dp.eq(0),
                pads.en.eq(digit_enable)
            ]

class HdmiGenerator(LiteXModule):
    def __init__(self, display_pads, output_pads, debug=None):
        self.extra_sink = stream.Endpoint(video_extra_data_layout)
        self.vtg_sink = stream.Endpoint(video_timing_layout)
        self.source   = stream.Endpoint(video_data_layout)

        self.nes_inputs = stream.Endpoint(nes_system_inputs)
        self.nes_outputs = stream.Endpoint(nes_system_outputs)

        self.comb += self.nes_inputs.enable.eq(self.extra_sink.ppu_enable)
        self.submodules.nes_system = NesSystem()
        self.comb += self.nes_inputs.connect(self.nes_system.inputs)
        self.comb += self.nes_system.outputs.connect(self.nes_outputs)
        self.comb += self.nes_inputs.valid.eq(1)
        self.comb += self.nes_outputs.ready.eq(1)

        self.submodules.display = SevenSegment(display_pads)
        self.comb += self.display.value.eq(self.extra_sink.last_ppu_enable_count[0:16])
        self.comb += output_pads[0].eq(self.extra_sink.ppu_enable)

        #self.submodules.nes_clock = ClockDomainsRenamer({"sys": "hdmi"})(NESClockScheduler())
        #self.comb += self.vtg_sink.connect(self.nes_clock.source)

        self.comb += self.vtg_sink.ready.eq(1)
        self.comb += self.source.r.eq(self.nes_outputs.r)
        self.comb += self.source.g.eq(self.nes_outputs.g)
        self.comb += self.source.b.eq(self.nes_outputs.b)
        self.comb += self.source.hsync.eq(self.vtg_sink.hsync)
        self.comb += self.source.vsync.eq(self.vtg_sink.vsync)
        self.comb += self.source.de.eq(self.vtg_sink.de)

        if debug is not None:
            self.comb += [
                debug.row.eq(self.extra_sink.ppu_count),
                debug.col.eq(self.extra_sink.ppu_vcount),
                debug.ppu_hcount.eq(self.nes_outputs.row),
                debug.ppu_vcount.eq(self.nes_outputs.col),
                debug.ppu_enable.eq(self.extra_sink.ppu_enable),
                debug.ppu_count.eq(self.extra_sink.last_ppu_enable_count),
                debug.ppu_count2.eq(self.extra_sink.ppu_enable_count),
            ]