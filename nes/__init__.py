import os

__dir__ = os.path.dirname(os.path.abspath(__file__))
data_location = os.path.join(__dir__, "hdl")


class Nes:
    def __init__(self, platform):
        files = [
            "lfsr.vhd",
            "clocked_sram_init.vhd",
            "clocked_sram.vhd",
            "delay_line.vhd",
            "frame_sync.vhd",
            "hdmi.vhd",
            "nes_cartridge.vhd",
            "nes_cpu.vhd",
            "nes_ppu.vhd",
            "nes.vhd",
            "resize_kernel.vhd",
            "sram.vhd",
            "uart.vhd",
            "wishbone_host_combiner.vhd",
            "nes_tripler.vhd",
            "edge_detect.vhd",
        ]

        for f in files:
            platform.add_source(os.path.join(data_location, f))