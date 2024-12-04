import os.path
__dir__ = os.path.split(os.path.abspath(os.path.realpath(__file__)))[0]
data_location = __dir__
src = "https://github.com/uglyoldbob/old_systems.git"

# Module version
version_str = "0.0.1"
version_tuple = (0, 0, 1, 0)
try:
    from packaging.version import Version as V
    pversion = V("0.0.1")
except ImportError:
    pass

# Data version info
data_version_str = "0.0.1"
data_version_tuple = (0, 0, 1)
try:
    from packaging.version import Version as V
    pdata_version = V("0.0.1")
except ImportError:
    pass
data_git_hash = "39d0cc9a3da7db730d0d2cc0161b8598151d7ed4"
data_git_describe = "v0.0.1"
data_git_msg = """\

"""

# Tool version info
tool_version_str = "0.0.1"
tool_version_tuple = (0, 0, 1)
try:
    from packaging.version import Version as V
    ptool_version = V("0.0.1")
except ImportError:
    pass

def data_file(f):
    """Get absolute path for file inside module."""
    fn = os.path.join(data_location, f)
    fn = os.path.abspath(fn)
    if not os.path.exists(fn):
        raise IOError("File {f} doesn't exist in old_systems".format(f))
    return fn

class Ddr3:
    def __init__(self, platform):
        platform.add_source(os.path.join(data_location, "clocked_sram_init.vhd"))
        platform.add_source(os.path.join(data_location, "clcoked_sram.vhd"))
        platform.add_source(os.path.join(data_location, "delay_line.vhd"))
        platform.add_source(os.path.join(data_location, "frame_sync.vhd"))
        platform.add_source(os.path.join(data_location, "hdmi.vhd"))
        platform.add_source(os.path.join(data_location, "nes_cartridge.vhd"))
        platform.add_source(os.path.join(data_location, "nes_cpu.vhd"))
        platform.add_source(os.path.join(data_location, "nes_ppu.vhd"))
        platform.add_source(os.path.join(data_location, "nes.vhd"))
        platform.add_source(os.path.join(data_location, "resize_kernel.vhd"))
        platform.add_source(os.path.join(data_location, "sram.vhd"))
        platform.add_source(os.path.join(data_location, "ddr3.v"), library="ddr3sim", language="SystemVerilog", copy=True)
        platform.add_source(os.path.join(data_location, "4096Mb_ddr3_parameters.vh"), library="ddr3sim", language="SystemVerilog", copy=True)
