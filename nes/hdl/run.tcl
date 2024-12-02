set_device -name GW2AR-18C GW2AR-LV18QN88C8/I7
set_option -vhdl_std vhd2008
add_file VexRiscv-verilog/VexRiscv_Linux.v
add_file lfsr.vhd
add_file src/tang_nano_nes.cst
add_file src/tang_nano_nes.sdc
add_file src/large_divider.vhd
add_file switch_debounce.vhd
add_file clocked_sram_init.vhd
add_file clocked_sram.vhd
add_file frame_sync.vhd
add_file resize_kernel.vhd
add_file nes_cpu.vhd
add_file nes_ppu.vhd
add_file nes_cartridge.vhd
add_file gowin_ddr.vhd
add_file gowin_nes_pll.vhd
add_file gowin_sdram.vhd
add_file src/gowin_sdram_interface.vhd
add_file gowin_video_fifo.vhd
add_file tmds_pll.vhd
add_file hdmi.vhd
add_file test_hdmi_out.v
add_file nes.vhd
add_file nes_tang_nano_20k.vhd
set_option -use_mspi_as_gpio 1
set_option -use_sspi_as_gpio 1
set_option -use_ready_as_gpio 1
set_option -use_done_as_gpio 1
set_option -rw_check_on_ram 1
run all