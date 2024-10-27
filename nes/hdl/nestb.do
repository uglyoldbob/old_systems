transcript on
if {[file exists rtl_work]} {
	vdel -lib rtl_work -all
}
vlib rtl_work
vmap work rtl_work

cp ../../rom_processor.tcl ./
cp ../../nestest.nes ./

source rom_processor.tcl

vcom -2008 -work work {../../nes_cpu.vhd}
vcom -2008 -work work {../../clocked_sram.vhd}
vcom -2008 -work work {../../nes_cartridge.vhd}
vcom -2008 -work work {../../nes.vhd}
vcom -2008 -work work {../../nestb.vhd}

vsim -t 1ps -L altera -L lpm -L sgate -L altera_mf -L altera_lnsim -L fiftyfivenm -L rtl_work -L work -voptargs="+acc"  nestb

add wave /nestb/nes/cpu/*

log * -r

restart -force

view structure
view signals
run 8us
wave zoom range 6us 8us