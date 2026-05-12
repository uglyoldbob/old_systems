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
    ("last_frame_cycle", 1),
    ("vblank", 1),
]

class NesSystem(LiteXModule):
    def __init__(self, debug=None):
        self.inputs = inputs = stream.Endpoint(nes_system_inputs)
        self.outputs = outputs = stream.Endpoint(nes_system_outputs)

        nes_ppu_hcount = Signal(9)  #0-340
        ppu_vcount = Signal(9)  #0-261
        odd_frame = Signal()

        fsm = FSM(reset_state="RUN")
        fsm = ResetInserter()(fsm)
        self.comb += fsm.reset.eq(0)
        self.submodules.fsm = fsm

        self.registers = registers = [Signal(8, name=f"registers_{i}") for i in range(8)]
        self.address_bit = address_bit = Signal()
        self.write_ignore_counter = write_ignore_counter = Signal(16)
        self.write_enabled = write_enabled = Signal()
        self.sim_done = Signal()

        if debug is not None:
            self.specials += Instance("sim_finish",
                i_clk=ClockSignal(),
                i_trigger=self.sim_done,
            )

        fsm.act("IDLE",
                NextValue(nes_ppu_hcount, 42),
                NextValue(ppu_vcount, 0),
                NextValue(odd_frame, 0),
                NextValue(address_bit, 0),
                NextValue(write_ignore_counter, 29658),
                NextValue(registers[0], 0),
                NextValue(registers[1], 0),
                NextValue(registers[2], 0),
                NextValue(registers[3], 0),
                NextValue(registers[4], 0),
                NextValue(registers[5], 0),
                NextValue(registers[6], 0),
                NextValue(registers[7], 0),
                NextValue(self.sim_done, 0),
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
                     If(odd_frame, 
                        NextValue(self.sim_done, 1)
                     ),
                  ),
               ),
               If(nes_ppu_hcount == 338, 
                  If(ppu_vcount == 261,
                     If(odd_frame,
                            NextValue(nes_ppu_hcount, nes_ppu_hcount + 2),
                        )
                     )
                  ),
               If(write_ignore_counter > 0, NextValue(write_ignore_counter, write_ignore_counter - 1)),
               outputs.r.eq(42),
               outputs.g.eq(42),
               outputs.b.eq(42),
            )       
        )
        self.comb += [
            If((ppu_vcount == 261) & (nes_ppu_hcount == 340), outputs.last_frame_cycle.eq(1)).Else(outputs.last_frame_cycle.eq(0)),
            If(write_ignore_counter == 0, write_enabled.eq(1)).Else(write_enabled.eq(0)),
            outputs.col.eq(nes_ppu_hcount),
            outputs.row.eq(ppu_vcount),
        ]

