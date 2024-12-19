transcript on
if ![file isdirectory nes_iputf_libs] {
	file mkdir nes_iputf_libs
}

if {[file exists rtl_work]} {
	vdel -lib rtl_work -all
}
vlib rtl_work
vmap work rtl_work

###### Libraries for IPUTF cores 
vlib nes_iputf_libs/max10_hdmi_output
vmap max10_hdmi_output ./nes_iputf_libs/max10_hdmi_output
###### End libraries for IPUTF cores 
###### MIF file copy and HDL compilation commands for IPUTF cores 

vcom -2008 -work work {/home/thomas/old_systems/nes/hdl/nes_cpu.vhd}
vcom -2008 -work work {/home/thomas/old_systems/nes/hdl/clocked_sram.vhd}
vcom -2008 -work work {/home/thomas/old_systems/nes/hdl/lfsr.vhd}
vcom -2008 -work work {/home/thomas/old_systems/nes/hdl/resize_kernel.vhd}
vcom -2008 -work work {/home/thomas/old_systems/nes/hdl/frame_sync.vhd}
vcom -2008 -work work {/home/thomas/old_systems/nes/hdl/nes_cartridge.vhd}
vcom -2008 -work work {/home/thomas/old_systems/nes/hdl/nes_ppu.vhd}
vcom -2008 -work work {/home/thomas/old_systems/nes/hdl/nes.vhd}

cp ../test_roms/other/nestest.nes ./nestest.nes

python ./nestest.py ../test_roms/other/nestest.log

source rom_processor.tcl

vlog -sv -work work {./hdmi/tmds_channel.sv}
vlog -sv -work work {./hdmi/source_product_description_info_frame.sv}
vlog -sv -work work {./hdmi/serializer.sv}
vlog -sv -work work {./hdmi/packet_picker.sv}
vlog -sv -work work {./hdmi/packet_assembler.sv}
vlog -sv -work work {./hdmi/auxiliary_video_information_info_frame.sv}
vlog -sv -work work {./hdmi/audio_sample_packet.sv}
vlog -sv -work work {./hdmi/audio_info_frame.sv}
vlog -sv -work work {./hdmi/audio_clock_regeneration_packet.sv}
vlog -sv -work work {./hdmi/hdmi.sv}
vlog -sv -work work {./VexRiscv-verilog/VexRiscv_Linux.v}
vlog +define+den4096Mb +define+sg125 -sv -work work {./ddr3.v}
vcom -2008 -work work {./clocked_sram_init.vhd}
vcom -2008 -work work {./frame_sync.vhd}
vcom -2008 -work work {./resize_kernel.vhd}
vcom -2008 -work work {./lfsr.vhd}
vcom -2008 -work work {./ddr.vhd}
vcom -2008 -work work {./hdmi.vhd}
vcom -2008 -work work {./nes_cpu.vhd}
vcom -2008 -work work {./nes_ppu.vhd}
vcom -2008 -work work {./clocked_sram.vhd}
vcom -2008 -work work {./nes_cartridge.vhd}
vcom -2008 -work work {./nes.vhd}
vcom -2008 -work work {./nestb.vhd}
vcom -2008 -work work {./src/large_divider.vhd}
vcom -2008 -work work {./src/gowin_sdram_interface.vhd}
vcom -2008 -work work {./switch_debounce.vhd}
vcom -2008 -work work {./gowin_ddr.vhd}
vlog -sv -work work {./test_hdmi_out.v}
vcom -2008 -work work {./nes_tang_nano_20k.vhd}
vcom -2008 -work work {./nes_tang_nano_20k_tb.vhd}

vsim -t 1ps -L altera -L lpm -L sgate -L altera_mf -L altera_lnsim -L fiftyfivenm -L rtl_work -L work -voptargs="+acc"  nes_tang_nano_20k_tb

add wave /nestb/nes/cpu/*
add wave /nestb/nes/cartridge/rom_wb*

radix signal sim:/nestb/nes/ppu_process_column unsigned
radix signal sim:/nestb/nes/ppu_process_row unsigned

log * -r

restart -force

view structure
view signals
run 10us
wave zoom range 0us 1us

run -all
