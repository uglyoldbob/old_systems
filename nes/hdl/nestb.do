transcript on
if {[file exists rtl_work]} {
	vdel -lib rtl_work -all
}
vlib rtl_work
vmap work rtl_work

python ../../nestest.py ../../../test_roms/other/nestest.log

cp ../../rom_processor.tcl ./

source rom_processor.tcl

vlog -sv -work work {../../hdmi/tmds_channel.sv}
vlog -sv -work work {../../hdmi/source_product_description_info_frame.sv}
vlog -sv -work work {../../hdmi/serializer.sv}
vlog -sv -work work {../../hdmi/packet_picker.sv}
vlog -sv -work work {../../hdmi/packet_assembler.sv}
vlog -sv -work work {../../hdmi/auxiliary_video_information_info_frame.sv}
vlog -sv -work work {../../hdmi/audio_sample_packet.sv}
vlog -sv -work work {../../hdmi/audio_info_frame.sv}
vlog -sv -work work {../../hdmi/audio_clock_regeneration_packet.sv}
vlog -sv -work work {../../hdmi/hdmi.sv}
vlog +define+den4096Mb +define+sg125 -sv -work work {../../ddr3.v}
vcom -2008 -work work {../../lfsr.vhd}
vcom -2008 -work work {../../ddr.vhd}
vcom -2008 -work work {../../hdmi.vhd}
vcom -2008 -work work {../../nes_cpu.vhd}
vcom -2008 -work work {../../nes_ppu.vhd}
vcom -2008 -work work {../../clocked_sram.vhd}
vcom -2008 -work work {../../nes_cartridge.vhd}
vcom -2008 -work work {../../nes.vhd}
vcom -2008 -work work {../../nestb.vhd}

vsim -t 1ps -L altera -L lpm -L sgate -L altera_mf -L altera_lnsim -L fiftyfivenm -L rtl_work -L work -voptargs="+acc"  nestb

add wave /nestb/hdmi2/pixel_clock
add wave /nestb/hdmi2/column
add wave /nestb/hdmi2/row
add wave /nestb/hdmi2/vsync
add wave /nestb/hdmi2/hsync
add wave /nestb/hdmi2/tmds_0
add wave /nestb/hdmi2/tmds_1
add wave /nestb/hdmi2/tmds_2
add wave /nestb/hdmi2/pixels


log * -r

restart -force

view structure
view signals
run -all