from migen import *
from migen.genlib.cdc import MultiReg
from litex.gen import *
from litex.soc.interconnect.csr import *
from litex.soc.interconnect import stream
from litex.soc.cores.video import video_data_layout
from litex.soc.cores.video import video_timing_layout
from litex.soc.cores.video import hbits, vbits, video_timings

nes_system_inputs = [
    ("enable", 1),
]

nes_system_outputs = [
    ("r", 8),
    ("g", 8),
    ("b", 8),
    ("row", 9),
    ("col", 9),
    ("blob_test", 1),
]

class NesSystem(LiteXModule):
    def __init__(self):
        self.inputs = inputs = stream.Endpoint(nes_system_inputs)
        self.outputs = outputs = stream.Endpoint(nes_system_outputs)

        nes_ppu_hcount = Signal(9)  #0-340
        ppu_vcount = Signal(9)  #0-261
        odd_frame = Signal()

        fsm = FSM(reset_state="RUN")
        fsm = ResetInserter()(fsm)
        self.comb += fsm.reset.eq(0)
        self.submodules.fsm = fsm

        fsm.act("IDLE",
                NextValue(nes_ppu_hcount, 42),
                NextValue(ppu_vcount, 0),
                NextValue(odd_frame, 0),
                NextState("RUN")
        )
        fsm.act("RUN",
            If(inputs.enable,
               NextValue(nes_ppu_hcount, nes_ppu_hcount + 1),
               If(nes_ppu_hcount == 340, 
                  NextValue(nes_ppu_hcount, 0),
                  NextValue(ppu_vcount, ppu_vcount + 1),
                  If(ppu_vcount == 261,
                     NextValue(ppu_vcount, 0),
                     NextValue(odd_frame, ~odd_frame),
                     )
               ),
               outputs.r.eq(42),
               outputs.g.eq(42),
               outputs.b.eq(42),
            )       
        )
        self.comb += [
            outputs.col.eq(nes_ppu_hcount),
            outputs.row.eq(ppu_vcount),
            outputs.blob_test.eq(inputs.enable),
        ]

