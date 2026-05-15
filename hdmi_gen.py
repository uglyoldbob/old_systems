from migen import *
from migen.genlib.cdc import MultiReg
from litex.build.io import SDROutput, DDROutput
from litex.gen import *
from litex.soc.interconnect.csr import *
from litex.soc.interconnect import stream
from litex.soc.cores.code_tmds import TMDSEncoder
from litex.soc.cores.video import video_data_layout
from litex.soc.cores.video import video_timing_layout
from litex.soc.cores.video import hbits, vbits, video_timings, VideoS7HDMI10to1Serializer

from .nes.hdl.nes import NesSystem, nes_system_inputs, nes_system_outputs

# ============================================================
# 32-bit LFSR
# Equivalent to VHDL entity: lfsr32
# ============================================================

class LFSR32(LiteXModule):
    def __init__(self):
        self.dout = Signal(32)

        # Internal state
        d     = Signal(32, reset=0x00000001)
        reset = Signal()
        e     = Signal()

        # Outputs
        self.comb += self.dout.eq(d)

        # e <= d(31) xnor d(21) xnor d(1) xnor d(0)
        #
        # XNOR of 4 bits:
        # result = ~(a ^ b ^ c ^ d)
        #
        self.comb += e.eq(~(d[31] ^ d[21] ^ d[1] ^ d[0]))

        # if d == 0 -> reset = 1
        self.comb += reset.eq(d == 0)

        # d <= d(30 downto 0) & (reset or e)
        self.sync += d.eq(Cat(reset | e, d[:31]))


# ============================================================
# 8-bit LFSR
# Equivalent to VHDL entity: lfsr8
# ============================================================

class LFSR8(LiteXModule):
    def __init__(self):
        self.dout = Signal(8)

        # Internal state
        d     = Signal(8, reset=0x01)
            
        reset = Signal()
        e     = Signal()

        # Outputs
        self.comb += self.dout.eq(d)

        # e <= d(7) xnor d(5) xnor d(4) xnor d(3)
        self.comb += e.eq(~(d[7] ^ d[5] ^ d[4] ^ d[3]))

        # Zero-state recovery
        self.comb += reset.eq(d == 0)

        # Shift
        self.sync += d.eq(Cat(reset | e, d[:7]))


# ============================================================
# 32-bit LFSR with High-Pass Filter
# Equivalent to VHDL entity: lfsr32_hpf
# ============================================================

class LFSR32HPF(LiteXModule):
    def __init__(self):
        self.hp_enable = Signal()
        self.dout      = Signal(32)

        # ====================================================
        # LFSR
        # ====================================================

        d     = Signal(32, reset=0x00000001)
        reset = Signal()
        e     = Signal()

        self.comb += [
            e.eq(~(d[31] ^ d[21] ^ d[1] ^ d[0])),
            reset.eq(d == 0),
        ]

        self.sync += d.eq(Cat(reset | e, d[:31]))

        # ====================================================
        # Filter State
        # ====================================================

        x_in  = Signal((32, True))
        x_z1  = Signal((32, True))
        y_z1  = Signal((32, True))
        y_out = Signal((32, True))

        self.comb += x_in.eq(d)

        # Temporary 33-bit signed intermediates
        diff = Signal((33, True))
        summ = Signal((33, True))
        ytmp = Signal((33, True))

        self.comb += [
            diff.eq(x_in - x_z1),
            summ.eq(y_z1 + diff),
            ytmp.eq(summ >> 1),   # alpha = 0.5
        ]

        # ====================================================
        # Filter Process
        # ====================================================

        self.sync += [
            If(~self.hp_enable,
                x_z1.eq(x_in),
                y_z1.eq(0),
                y_out.eq(x_in)
            ).Else(
                x_z1.eq(x_in),
                y_z1.eq(ytmp[:32]),
                y_out.eq(ytmp[:32]),
            )
        ]

        # ====================================================
        # Output Select
        # ====================================================

        self.comb += If(
            self.hp_enable,
            self.dout.eq(y_out)
        ).Else(
            self.dout.eq(d)
        )

hdmi_phy_layout = [
    ("d0", 10),
    ("d1", 10),
    ("d2", 10),
    ("mode", 3),
]

hdmi_video_timing_layout = [
    # Synchronization signals.
    ("hsync", 1),
    ("vsync", 1),
    ("de",    1),
    ("mode",    3),
    # Extended/Optional synchronization signals.
    ("hres",   hbits),
    ("vres",   vbits),
    ("hcount", hbits),
    ("vcount", vbits),
]

hdmi_video_data_layout = [
    # Synchronization signals.
    ("hsync", 1),
    ("vsync", 1),
    ("mode",  3),
    # Data signals.
    ("r",     8),
    ("g",     8),
    ("b",     8),
    ("ctl",   4),
    ("aux",  10),
]

MODE_VIDEO = 0
MODE_VIDEO_GUARD = 1
MODE_DATA_GUARD = 2
MODE_DATA = 3
MODE_DATA_PREAMBLE = 4
MODE_VIDEO_PREAMBLE = 5
MODE_CONTROL = 6

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

terc4_table = {
    0x0: 0b1010011100,
    0x1: 0b1001100011,
    0x2: 0b1011100100,
    0x3: 0b1011100010,
    0x4: 0b0101110001,
    0x5: 0b0100011110,
    0x6: 0b0110001110,
    0x7: 0b0100111100,
    0x8: 0b1011001100,
    0x9: 0b0100111001,
    0xA: 0b0110011100,
    0xB: 0b1011000110,
    0xC: 0b1010001110,
    0xD: 0b1001110001,
    0xE: 0b0101100011,
    0xF: 0b1011000011,
}

class HdmiControl(Module):
    def __init__(self):
        self.d = Signal(2)
        self.q = Signal(10)
        self.comb += Case(self.d, {
            0: self.q.eq(0b1101010100),
            1: self.q.eq(0b0010101011),
            2: self.q.eq(0b0101010100),
            3: self.q.eq(0b1010101011),
        })

class TERC4Encoder(Module):
    def __init__(self):
        self.d = Signal(4)
        self.q = Signal(10)

        cases = {}
        for k, v in terc4_table.items():
            cases[k] = self.q.eq(v)

        self.comb += Case(self.d, cases)

class NesTimingGenerator(LiteXModule):
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
        
        self._pre_h_start = CSRStorage(hbits, 256 - 7, description="Start of processing for smaller image")
        self._pre_h_stop = CSRStorage(hbits, 256 + 341*3 - 7, description="End of processing for smaller image")
        self._hsync_start = CSRStorage(hbits, 7 + vt["h_active"] + vt["h_sync_offset"],
            description="Horizontal sync start.")
        self._ppu_vres        = CSRStorage(vbits, vt["v_active"] - 3,
            description="Vertical active resolution.")
        self._vscan       = CSRStorage(vbits, vt["v_active"] + vt["v_blanking"] - 1,
            description="Vertical scan period.")

        self.pre_h_start = pre_h_start = Signal(hbits)
        self.pre_h_stop = pre_h_stop = Signal(hbits)
        self.hsync_start = hsync_start = Signal(hbits)
        self.ppu_vres        = ppu_vres        = Signal(vbits)
        self.vscan       = vscan       = Signal(vbits)
        self.specials += MultiReg(self._pre_h_start.storage, pre_h_start)
        self.specials += MultiReg(self._pre_h_stop.storage,  pre_h_stop)
        self.specials += MultiReg(self._hsync_start.storage, hsync_start)
        self.specials += MultiReg(self._ppu_vres.storage,        ppu_vres)
        self.specials += MultiReg(self._vscan.storage,       vscan)

        phactive = Signal()
        pvactive = Signal()
        pvactive_fast = Signal()
        self.nes_ppu_end = Signal()
        self.vtg_sink = vtg_sink = stream.Endpoint(hdmi_video_timing_layout)
        self.forward = forward = stream.Endpoint(hdmi_video_timing_layout)
        self.esource = esource = stream.Endpoint(video_extra_data_layout)
        self.fsm = fsm = FSM(reset_state="IDLE")
        self.nes_frame_done = nes_frame_done = Signal()
        self.last_cycle_done = last_cycle_done = Signal()

        self.sync += [
            vtg_sink.ready.eq(forward.ready),
        ]

        fsm.act("IDLE",
            NextValue(esource.ppu_col,  0),
            NextValue(esource.ppu_row,  0),
            NextValue(esource.ppu_count,  0),
            NextValue(esource.ppu_vcount,  0),
            NextValue(esource.ppu_enable_count, 0),
            NextValue(nes_frame_done, 0),
            NextValue(last_cycle_done, 0),
            NextState("RUN")
        )
        fsm.act("RUN",
            forward.hsync.eq(vtg_sink.hsync),
            forward.vsync.eq(vtg_sink.vsync),
            forward.de.eq(vtg_sink.de),
            forward.hres.eq(vtg_sink.hres),
            forward.vres.eq(vtg_sink.vres),
            forward.hcount.eq(vtg_sink.hcount),
            forward.vcount.eq(vtg_sink.vcount),
            forward.mode.eq(vtg_sink.mode),
            If(self.nes_ppu_end,        NextValue(nes_frame_done,       1)),
            If(vtg_sink.ready,
                If(vtg_sink.hcount == pre_h_start, NextValue(phactive, 1)),
                If(vtg_sink.hcount == pre_h_stop,  NextValue(phactive, 0)),
                If(phactive, NextValue(esource.ppu_count, esource.ppu_count + 1))
                    .Else(NextValue(esource.ppu_count, 0)),
                If(esource.ppu_count == 2, 
                    NextValue(esource.ppu_count, 0),
                    NextValue(esource.ppu_col, esource.ppu_col + 1),
                ),
                If(esource.ppu_enable, NextValue(esource.ppu_enable_count, esource.ppu_enable_count + 1)),

                If(vtg_sink.hcount == hsync_start,
                    # Increment VCount.
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
                    If(vtg_sink.vcount == 0,           NextValue(pvactive,       1)),
                    If(vtg_sink.vcount == ppu_vres,        NextValue(pvactive_fast,       1)),
                    If(vtg_sink.vcount == ppu_vres,        NextValue(pvactive,       0)),
                    If(vtg_sink.vcount == vscan,
                       NextValue(esource.last_ppu_enable_count, esource.ppu_enable_count),
                        NextValue(nes_frame_done, 0),
                        NextValue(esource.ppu_enable_count, 0),
                    ),
                )
            )
        )
        self.sync += [
            self.esource.row_process.eq(phactive),
            self.esource.col_process.eq(pvactive),
            self.esource.fast.eq(pvactive_fast),
        ]

        self.comb += self.esource.process.eq(phactive & pvactive)
        self.comb += If((esource.ppu_vcount == 1) & (esource.ppu_count == 1),
            self.esource.ppu_enable.eq(1)).Else(self.esource.ppu_enable.eq(0))


class HdmiVideoTimingGenerator(LiteXModule):
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

        # HDMI timing constants
        PREAMBLE_PIXELS = 8
        GUARD_PIXELS    = 2

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

        # Video Timing Source
        self.source = source = stream.Endpoint(hdmi_video_timing_layout)

        # # #

        # Resynchronize Enable to Video clock domain.
        self.enable = enable = Signal()
        self.specials += MultiReg(self._enable.storage, enable)

        # Resynchronize Horizontal Timings to Video clock domain.
        self.hres        = hres        = Signal(hbits)
        self.hsync_start = hsync_start = Signal(hbits)
        self.hsync_end   = hsync_end   = Signal(hbits)
        self.hscan       = hscan       = Signal(hbits)
        self.specials += MultiReg(self._hres.storage,        hres)
        self.specials += MultiReg(self._hsync_start.storage, hsync_start)
        self.specials += MultiReg(self._hsync_end.storage,   hsync_end)
        self.specials += MultiReg(self._hscan.storage,       hscan)

        # Resynchronize Vertical Timings to Video clock domain.
        self.vres        = vres        = Signal(vbits)
        self.vsync_start = vsync_start = Signal(vbits)
        self.vsync_end   = vsync_end   = Signal(vbits)
        self.vscan       = vscan       = Signal(vbits)
        self.specials += MultiReg(self._vres.storage,        vres)
        self.specials += MultiReg(self._vsync_start.storage, vsync_start)
        self.specials += MultiReg(self._vsync_end.storage,   vsync_end)
        self.specials += MultiReg(self._vscan.storage,       vscan)

        self.ihcount = Signal(hbits)

        # HDMI mode transition points:
        #
        #  hcount: 0                  hres             hscan-9     hscan-1  hscan
        #          |<----- VIDEO ----->|<--- CONTROL --->|<-PREAMBLE->|<-GRD->|  (wraps to 0)
        #
        pre_preamble = Signal(hbits)  # hscan - 9 : first VIDEO_PREAMBLE pixel
        pre_guard    = Signal(hbits)  # hscan - 1 : first VIDEO_GUARD pixel

        self.comb += [
            pre_preamble.eq(hscan - (PREAMBLE_PIXELS + GUARD_PIXELS) + 1),
            pre_guard.eq(hscan - GUARD_PIXELS + 1),
        ]

        # Generate timings.
        hactive = Signal()
        vactive = Signal()
        fsm = FSM(reset_state="IDLE")
        fsm = ResetInserter()(fsm)
        self.fsm = fsm
        self.comb += fsm.reset.eq(~enable)
        fsm.act("IDLE",
            NextValue(hactive,        0),
            NextValue(vactive,        0),
            NextValue(source.hres,    hres),
            NextValue(source.vres,    vres),
            NextValue(source.hcount,  vt["h_active"] + vt["h_blanking"] - 8),
            NextValue(self.ihcount,   0),
            NextValue(source.vcount,  0),
            NextValue(source.mode,    MODE_CONTROL),
            NextState("RUN")
        )
        self.comb += source.de.eq(hactive & vactive)
        self.sync += source.first.eq((source.hcount ==     0) & (source.vcount ==     0)),
        self.sync += source.last.eq( (source.hcount == hscan) & (source.vcount == vscan)),
        fsm.act("RUN",
            source.valid.eq(1),
            If(source.ready,
                # Increment HCount.
                NextValue(source.hcount, source.hcount + 1),

                # -----------------------------------------------------------------
                # HDMI horizontal mode sequencing (per line):
                #   CONTROL → VIDEO_PREAMBLE(8) → VIDEO_GUARD(2) → VIDEO → CONTROL
                # -----------------------------------------------------------------
                If(source.hcount == 0,
                    # First active pixel: enter VIDEO mode.
                    NextValue(hactive,       1),
                    NextValue(source.mode,   MODE_VIDEO),
                ),
                If(source.hcount == hres,
                    # End of active video: straight to CONTROL (no post-active guard).
                    NextValue(hactive,       0),
                    NextValue(source.mode,   MODE_CONTROL),
                ),
                If(source.hcount == pre_preamble,
                    # Start of 8-pixel VIDEO_PREAMBLE before next active line.
                    NextValue(source.mode,   MODE_VIDEO_PREAMBLE),
                ),
                If(source.hcount == pre_guard,
                    # Start of 2-pixel VIDEO_GUARD immediately before active video.
                    NextValue(source.mode,   MODE_VIDEO_GUARD),
                ),
                If(source.hcount == hscan,
                    # End of scan line: wrap hcount back to 0.
                    NextValue(source.hcount, 0),
                ),

                # Generate HSync.
                If(source.hcount == hsync_start, NextValue(source.hsync, 1)),
                If(source.hcount == hsync_end,   NextValue(source.hsync, 0)),

                If(source.hcount == hsync_start,
                    # Increment VCount.
                    NextValue(source.vcount, source.vcount + 1),
                    # Generate VActive / VSync.
                    If(source.vcount == 0,           NextValue(vactive,       1)),
                    If(source.vcount == vres,        NextValue(vactive,       0)),
                    If(source.vcount == vsync_start, NextValue(source.vsync,  1)),
                    If(source.vcount == vsync_end,   NextValue(source.vsync,  0)),
                    If(source.vcount == vscan,       NextValue(source.vcount, 0)),
                )
            )
        )

class VideoGenericHdmiEncoder(LiteXModule):
    def __init__(self, clock_domain="sys"):
        self.sink   = sink   = stream.Endpoint(hdmi_video_data_layout)
        self.source = source = stream.Endpoint(hdmi_phy_layout)

        # Always ack Sink, no backpressure.
        self.comb += sink.ready.eq(1)

        # Submodules.
        control = ClockDomainsRenamer(clock_domain)(HdmiControl())
        encoders = [ClockDomainsRenamer(clock_domain)(TMDSEncoder()) for _ in range(3)]
        tercs    = [ClockDomainsRenamer(clock_domain)(TERC4Encoder()) for _ in range(3)]
        self.submodules += control, *encoders, *tercs

        # Unpack for readability.
        encoder0, encoder1, encoder2 = encoders
        terc0,    terc1,    terc2    = tercs

        # Channel inputs (sink color → encoder data).
        for enc, color in zip(encoders, [sink.b, sink.g, sink.r]):
            self.comb += enc.d.eq(color)

        self.comb += [
            source.mode.eq(sink.mode),
            control.d.eq(Cat(sink.hsync, sink.vsync)),
        ]

        # Helper to drive all encoders' de/c signals and all tercs' d signals.
        def drive_all(de, c0, c1, c2, t0=0, t1=0, t2=0):
            return [
                *[enc.de.eq(de) for enc in encoders],
                encoder0.c.eq(c0), encoder1.c.eq(c1), encoder2.c.eq(c2),
                terc0.d.eq(t0),    terc1.d.eq(t1),    terc2.d.eq(t2),
            ]

        self.comb += Case(sink.mode, {
            MODE_VIDEO: [
                *drive_all(de=1, c0=0, c1=0, c2=0),
                source.d0.eq(encoder0.out),
                source.d1.eq(encoder1.out),
                source.d2.eq(encoder2.out),
            ],
            MODE_DATA: [
                *drive_all(de=0, c0=0, c1=0, c2=0,
                    t0=Cat(sink.hsync, sink.vsync, sink.aux[0:2]),
                    t1=sink.aux[2:6],
                    t2=sink.aux[6:10],
                ),
                source.d0.eq(terc0.q),
                source.d1.eq(terc1.q),
                source.d2.eq(terc2.q),
            ],
            MODE_CONTROL: [
                *drive_all(de=0, c0=0, c1=0b10, c2=0b00),
                source.d0.eq(control.q),
                source.d1.eq(0b1101010100),
                source.d2.eq(0b1101010100),
            ],
            MODE_VIDEO_PREAMBLE: [
                *drive_all(de=0, c0=0, c1=0b10, c2=0b00,
                    t0=Cat(sink.hsync, sink.vsync, 1, 1),
                ),
                source.d0.eq(terc0.q),
                source.d1.eq(encoder1.out),
                source.d2.eq(encoder2.out),
            ],
            MODE_DATA_PREAMBLE: [
                *drive_all(de=0, c0=0, c1=0b10, c2=0b10,
                    t0=Cat(sink.hsync, sink.vsync, 1, 1),
                ),
                source.d0.eq(terc0.q),
                source.d1.eq(encoder1.out),
                source.d2.eq(encoder2.out),
            ],
            MODE_VIDEO_GUARD: [
                *drive_all(de=0, c0=0, c1=0, c2=0),
                source.d0.eq(0b1011001100),
                source.d1.eq(0b0100110011),
                source.d2.eq(0b1011001100),
            ],
            MODE_DATA_GUARD: [
                *drive_all(de=0, c0=0, c1=0, c2=0,
                    t0=Cat(sink.hsync, sink.vsync, 1, 1),
                ),
                source.d0.eq(terc0.q),
                source.d1.eq(0b0100110011),
                source.d2.eq(0b0100110011),
            ],
        })


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

class VideoS7HDMIPHY2(LiteXModule):
    def __init__(self, pads, clock_domain="sys"):
        self.sink = sink = stream.Endpoint(hdmi_phy_layout)
        # # #

        # Always ack Sink, no backpressure.
        self.comb += sink.ready.eq(1)

        # Clocking + Differential Signaling.
        pads_clk = Signal()
        self.specials += DDROutput(i1=1, i2=0, o=pads_clk, clk=ClockSignal(clock_domain))
        self.specials += Instance("OBUFDS", i_I=pads_clk, o_O=pads.clk_p, o_OB=pads.clk_n)

        # Data channels.
        for i, data_i in enumerate([sink.d0, sink.d1, sink.d2]):
            pad_o = Signal(name=f"pad_o{i}")
            serializer = VideoS7HDMI10to1Serializer(
                data_i       = data_i,
                data_o       = pad_o,
                clock_domain = clock_domain,
            )
            setattr(self.submodules, f"serializer{i}", serializer)
            self.specials += Instance("OBUFDS",
                i_I  = pad_o,
                o_O  = getattr(pads, f"data{i}_p"),
                o_OB = getattr(pads, f"data{i}_n"),
            )

class HdmiGenerator(LiteXModule):
    def __init__(self, display_pads, debug=None):
        # Endpoints.
        self.extra_sink  = stream.Endpoint(video_extra_data_layout)
        self.vtg_sink    = stream.Endpoint(hdmi_video_timing_layout)
        self.source      = stream.Endpoint(hdmi_video_data_layout)
        self.nes_inputs  = stream.Endpoint(nes_system_inputs)
        self.nes_outputs = stream.Endpoint(nes_system_outputs)

        # Submodules.
        self.submodules.nes_system = NesSystem(debug)
        self.submodules.display    = SevenSegment(display_pads)
        self.submodules.random     = LFSR32HPF()

        # NES system connections.
        self.comb += [
            self.nes_inputs.enable.eq(self.extra_sink.ppu_enable),
            self.nes_inputs.valid.eq(1),
            self.nes_inputs.connect(self.nes_system.inputs),
            self.nes_system.outputs.connect(self.nes_outputs),
            self.nes_outputs.ready.eq(1),
        ]

        # Display.
        self.comb += self.display.value.eq(self.extra_sink.last_ppu_enable_count[0:16])

        # Random noise generator.
        self.comb += self.random.hp_enable.eq(1)

        # Video output.
        self.comb += [
            self.vtg_sink.ready.eq(1),
            self.source.r.eq(self.random.dout[0:8]),
            self.source.g.eq(self.random.dout[8:16]),
            self.source.b.eq(self.random.dout[16:24]),
            self.source.hsync.eq(self.vtg_sink.hsync),
            self.source.vsync.eq(self.vtg_sink.vsync),
            self.source.mode.eq(self.vtg_sink.mode),
        ]

        # Debug.
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