transcript on
if {[file exists rtl_work]} {
	vdel -lib rtl_work -all
}
vlib rtl_work
vmap work rtl_work

python ../../nestest.py ../../../test_roms/other/nestest.log

cp ../../rom_processor.tcl ./
cp ../../../test_roms/other/nestest.nes ./nestest.nes
mkdir -p ./riscv-bios-rust
cp ../../riscv-bios-rust/combios.dat ./riscv-bios-rust/combios.dat

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
vlog -sv -work work {../../VexRiscv-verilog/VexRiscv_Min.v}
vlog +define+den4096Mb +define+sg125 -sv -work work {../../ddr3.v}
vcom -2008 -work work {../../clocked_sram_init.vhd}
vcom -2008 -work work {../../frame_sync.vhd}
vcom -2008 -work work {../../resize_kernel.vhd}
vcom -2008 -work work {../../lfsr.vhd}
vcom -2008 -work work {../../ddr.vhd}
vcom -2008 -work work {../../hdmi.vhd}
vcom -2008 -work work {../../nes_cpu.vhd}
vcom -2008 -work work {../../nes_ppu.vhd}
vcom -2008 -work work {../../clocked_sram.vhd}
vcom -2008 -work work {../../nes_cartridge.vhd}
vcom -2008 -work work {../../edge_detect.vhd}
vcom -2008 -work work {../../nes_tripler.vhd}
vcom -2008 -work work {../../wishbone_host_combiner.vhd}
vcom -2008 -work work {../../uart.vhd}
vcom -2008 -work work {../../nes.vhd}
vcom -2008 -work work {../../nestb.vhd}

vsim -t 1ps -L altera -L lpm -L sgate -L altera_mf -L altera_lnsim -L fiftyfivenm -L rtl_work -L work -voptargs="+acc"  nestb

add wave /nestb/nes/cpugen/wbc/*
add wave /nestb/nes/cpugen/serial/*

#radix signal sim:/nestb/nes/ppu_process_column unsigned
#radix signal sim:/nestb/nes/ppu_process_row unsigned

log * -r

restart -force

view structure
view signals
run 10us
wave zoom range 0us 1us

run -all