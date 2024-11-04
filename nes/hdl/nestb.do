transcript on
if {[file exists rtl_work]} {
	vdel -lib rtl_work -all
}
vlib rtl_work
vmap work rtl_work

python ../../nestest.py ../../../test_roms/other/nestest.log

cp ../../rom_processor.tcl ./

source rom_processor.tcl

vcom -2008 -work work {../../nes_cpu.vhd}
vcom -2008 -work work {../../nes_ppu.vhd}
vcom -2008 -work work {../../clocked_sram.vhd}
vcom -2008 -work work {../../nes_cartridge.vhd}
vcom -2008 -work work {../../nes.vhd}
vcom -2008 -work work {../../nestb.vhd}

vsim -t 1ps -L altera -L lpm -L sgate -L altera_mf -L altera_lnsim -L fiftyfivenm -L rtl_work -L work -voptargs="+acc"  nestb

add wave /nestb/nes/ppu/*
add wave /nestb/nes/cpu/*

log * -r

restart -force

view structure
view signals
run -all
wave zoom range 8us 10us