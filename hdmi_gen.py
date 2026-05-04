from migen import *
from litex.gen import *
from litex.soc.interconnect import stream
from litex.soc.cores.video import video_data_layout
from litex.soc.cores.video import video_timing_layout


class HdmiGenerator(LiteXModule):
    def __init__(self, default_video_timings="800x600@60Hz"):
        self.dummy = default_video_timings
        self.vtg_sink = stream.Endpoint(video_timing_layout)
        self.source   = stream.Endpoint(video_data_layout)
        self.data = Signal(32)
        self.specials += Instance("lfsr32_hpf",
            i_clock     = ClockSignal("hdmi"),
            i_hp_enable = 1,
            o_dout    = self.data,
        )
        self.comb += self.vtg_sink.ready.eq(1)
        self.comb += self.source.r.eq(self.data[0:8])
        self.comb += self.source.g.eq(self.data[8:16])
        self.comb += self.source.b.eq(self.data[16:24])
        self.comb += self.source.hsync.eq(self.vtg_sink.hsync)
        self.comb += self.source.vsync.eq(self.vtg_sink.vsync)
        self.comb += self.source.de.eq(self.vtg_sink.de)
