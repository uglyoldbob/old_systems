#!/usr/bin/env python3

#
# This file is part of LiteX-Boards.
#
# Copyright (c) 2018-2019 Rohit Singh <rohit@rohitksingh.in>
# Copyright (c) 2020 Feliks Montez <feliks.montez@gmail.com>
# SPDX-License-Identifier: BSD-2-Clause

# python3 -m old_systems.nes_numato --build

from migen import *

from litex.gen import *

from litex_boards.platforms import numato_mimas_a7

from litex.soc.cores.clock import *
from litex.soc.integration.soc_core import *
from litex.soc.integration.builder import *
from litex.soc.cores.led import LedChaser
from litex.soc.cores.video import VideoS7HDMIPHY
from litex.soc.interconnect import stream

from litedram.modules import MT41J128M16
from litedram.phy import s7ddrphy

from liteeth.phy.s7rgmii import LiteEthPHYRGMII
from .hdmi_gen import HdmiGenerator, VideoTimingGenerator
from .nes import Nes

# CRG ----------------------------------------------------------------------------------------------

class _CRG(LiteXModule):
    def __init__(self, platform, sys_clk_freq):
        self.rst          = Signal()
        self.cd_sys       = ClockDomain()
        self.cd_sys4x     = ClockDomain()
        self.cd_sys4x_dqs = ClockDomain()
        self.cd_idelay    = ClockDomain()
        self.cd_hdmi = ClockDomain();
        self.cd_hdmi5 = ClockDomain();
        self.reset = platform.request("cpu_reset") | self.rst
        self.clock_in = platform.request("clk100")
        # # #

        self.pll = pll = S7PLL(speedgrade=-1)
        self.comb += pll.reset.eq(self.reset)
        pll.register_clkin(self.clock_in, 100e6)
        pll.create_clkout(self.cd_sys,       sys_clk_freq)
        pll.create_clkout(self.cd_sys4x,     4*sys_clk_freq)
        pll.create_clkout(self.cd_sys4x_dqs, 4*sys_clk_freq, phase=90)
        pll.create_clkout(self.cd_idelay,    200e6)
        platform.add_false_path_constraints(self.cd_sys.clk, pll.clkin) # Ignor
        
        self.pll2 = pll2 = S7PLL(speedgrade=-1)
        self.comb += pll2.reset.eq(self.reset)
        pll2.register_clkin(self.clock_in, 100e6)
        pll2.create_clkout(self.cd_hdmi, 74.25e6)
        pll2.create_clkout(self.cd_hdmi5, 371.25e6)
        platform.add_false_path_constraints(self.cd_sys.clk, pll2.clkin) # Ignore sys_clk to pll.clkin path created by SoC's rst.

        self.idelayctrl = S7IDELAYCTRL(self.cd_idelay)

# BaseSoC ------------------------------------------------------------------------------------------

class BaseSoC(SoCCore):
    def add_video_generator(self, display_pads, output_pads, name="video_generator", phy=None, timings="1280x720@60Hz", clock_domain="sys"):
        generator = HdmiGenerator(display_pads, output_pads)

        # Video Timing Generator.
        self.check_if_exists(f"{name}_vtg")
        vtg = VideoTimingGenerator(default_video_timings=timings if isinstance(timings, str) else timings[1])
        vtg = ClockDomainsRenamer(clock_domain)(vtg)
        self.add_module(name=f"{name}_vtg", module=vtg)

        # ColorsBars Pattern.
        self.check_if_exists(name)
        self.add_module(name=name, module=generator)

        # Connect Video Timing Generator to ColorsBars Pattern.
        self.comb += [
            vtg.source.connect(generator.vtg_sink),
            vtg.esource.connect(generator.extra_sink),
            generator.source.connect(phy if isinstance(phy, stream.Endpoint) else phy.sink)
        ]

    def __init__(self, sys_clk_freq=100e6,
        with_led_chaser = True,
        with_ethernet   = False,
        eth_ip          = "192.168.1.50",
        remote_ip       = None,
        eth_dynamic_ip  = False,
        **kwargs):
        platform = numato_mimas_a7.Platform()

        test = platform.request("test")

        nes = Nes(platform)

        # CRG --------------------------------------------------------------------------------------
        self.crg = _CRG(platform, sys_clk_freq)

        # SoCCore ----------------------------------------------------------------------------------
        SoCCore.__init__(self, platform, sys_clk_freq, ident="LiteX SoC on Mimas A7", **kwargs)

        # DDR3 SDRAM -------------------------------------------------------------------------------
        if not self.integrated_main_ram_size:
            self.ddrphy = s7ddrphy.A7DDRPHY(platform.request("ddram"),
                memtype      = "DDR3",
                nphases      = 4,
                sys_clk_freq = sys_clk_freq)
            self.add_sdram("sdram",
                phy           = self.ddrphy,
                module        = MT41J128M16(sys_clk_freq, "1:4"),
                l2_cache_size = kwargs.get("l2_size", 8192)
            )

        # Ethernet ---------------------------------------------------------------------------------
        if with_ethernet:
            self.ethphy = LiteEthPHYRGMII(
                clock_pads = self.platform.request("eth_clocks"),
                pads       = self.platform.request("eth"))
            self.add_ethernet(phy=self.ethphy, dynamic_ip=eth_dynamic_ip, local_ip=eth_ip, remote_ip=remote_ip)

        # Leds -------------------------------------------------------------------------------------
        if with_led_chaser:
            self.leds = LedChaser(
                pads         = platform.request_all("user_led"),
                sys_clk_freq = sys_clk_freq)
        
        hdmi_phy = VideoS7HDMIPHY(self.platform.request("hdmi_out"))
        self.submodules.hdmi_phy = ClockDomainsRenamer({"sys": "hdmi", "sys5x": "hdmi5"})(hdmi_phy)
        
        self.add_video_generator(platform.request("seven_segment"), test, "hdmi_out", self.hdmi_phy, clock_domain="hdmi")

# Build --------------------------------------------------------------------------------------------

def main():
    from litex.build.parser import LiteXArgumentParser
    parser = LiteXArgumentParser(platform=numato_mimas_a7.Platform, description="LiteX SoC on Mimas A7.")
    parser.add_argument("--sim", action="store_true", help="Simulate the design")
    parser.add_target_argument("--sys-clk-freq",   default=100e6, type=float, help="System clock frequency.")
    parser.add_target_argument("--with-ethernet",  action="store_true",       help="Enable Ethernet support.")
    parser.add_target_argument("--eth-ip",         default="192.168.1.50",    help="Ethernet/Etherbone IP address.")
    parser.add_target_argument("--remote-ip",      default="192.168.1.100",   help="Remote IP address of TFTP server.")
    parser.add_target_argument("--eth-dynamic-ip", action="store_true",       help="Enable dynamic Ethernet IP assignment.")
    args = parser.parse_args()

    soc = BaseSoC(
        sys_clk_freq   = args.sys_clk_freq,
        with_ethernet  = args.with_ethernet,
        eth_ip         = args.eth_ip,
        eth_dynamic_ip = args.eth_dynamic_ip,
        remote_ip      = args.remote_ip,
        **parser.soc_argdict
    )
    builder = Builder(soc, **parser.builder_argdict)
    if args.build:
        builder.build(**parser.toolchain_argdict)

    if args.load:
        prog = soc.platform.create_programmer()
        prog.load_bitstream(builder.get_bitstream_filename(mode="sram"))

if __name__ == "__main__":
    main()
