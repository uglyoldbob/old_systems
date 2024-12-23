//Copyright (C)2014-2024 GOWIN Semiconductor Corporation.
//All rights reserved.
//File Title: Timing Constraints file
//Tool Version: V1.9.10.03 
//Created Time: 2024-12-23 13:14:04
create_clock -name input_clock -period 37.037 -waveform {0 18.518} [get_ports {clock}]
create_generated_clock -name clockb -source [get_nets {hdmi_pixel_clock}] -master_clock hdmi_clock -divide_by 12 -phase 180 [get_nets {nes/cpu/clockb}]
create_generated_clock -name clocka -source [get_nets {hdmi_pixel_clock}] -master_clock hdmi_clock -divide_by 12 [get_nets {nes/cpu/clocka}]
create_generated_clock -name memclk -source [get_nets {hdmi_pixel_clock}] -master_clock hdmi_clock -divide_by 12 -phase 30 [get_nets {nes/memory_clock}]
create_generated_clock -name hdmi_clock -source [get_nets {tmds_clock}] -master_clock tmds_clock -divide_by 5 [get_nets {hdmi_pixel_clock}]
create_generated_clock -name tmds_clock -source [get_ports {clock}] -master_clock input_clock -divide_by 4 -multiply_by 55 [get_nets {tmds_clock}]
create_generated_clock -name ppuclk -source [get_nets {hdmi_pixel_clock}] -master_clock hdmi_clock -divide_by 4 [get_nets {nes/ppu_clock}]
set_clock_latency -source 100 [get_clocks {input_clock}] 
