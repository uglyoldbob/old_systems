//Copyright (C)2014-2024 GOWIN Semiconductor Corporation.
//All rights reserved.
//File Title: Timing Constraints file
//Tool Version: V1.9.10.03 
//Created Time: 2024-12-01 20:34:10
create_clock -name input_clock -period 37.037 -waveform {0 18.518} [get_ports {clock}]
create_generated_clock -name pixel_clock -source [get_nets {tmds_clock}] -master_clock tmds_clock -divide_by 5 [get_nets {hdmi_pixel_clock}]
create_generated_clock -name tmds_clock -source [get_ports {clock}] -master_clock input_clock -divide_by 4 -multiply_by 55 [get_nets {tmds_clock}]
create_generated_clock -name clock_d -source [get_nets {clock}] -master_clock input_clock -divide_by 1 -multiply_by 1 [get_nets {clock_d}]
create_generated_clock -name nes_clock -source [get_nets {hdmi_pixel_clock}] -master_clock pixel_clock -divide_by 3 [get_nets {nes_clock}]
create_generated_clock -name nes_mem_clk -source [get_nets {nes_clock}] -master_clock nes_clock -divide_by 12 [get_nets {nes/memory_clock}]
create_generated_clock -name cpu_clocka -source [get_nets {nes_clock}] -master_clock nes_clock -divide_by 12 [get_nets {nes/cpu/clocka}]
create_generated_clock -name button_clock -source [get_nets {hdmi_pixel_clock}] -master_clock pixel_clock -divide_by 1048576 [get_nets {button_clock}]
create_generated_clock -name nes_memory_clock -source [get_nets {nes_clock}] -master_clock nes_clock -divide_by 12 [get_nets {nes/cpu_memory_clock}]
create_generated_clock -name nes_ppu_clock -source [get_nets {nes_clock}] -master_clock nes_clock -divide_by 12 [get_nets {nes/ppu_clock}]
