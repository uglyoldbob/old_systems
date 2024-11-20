//Copyright (C)2014-2024 GOWIN Semiconductor Corporation.
//All rights reserved.
//File Title: Timing Constraints file
//Tool Version: V1.9.9.03  Education
//Created Time: 2024-11-19 12:25:38
create_clock -name input_clock -period 37.037 -waveform {0 18.518} [get_ports {clock}]
create_generated_clock -name tmds_clock -source [get_ports {clock}] -master_clock input_clock -divide_by 4 -multiply_by 55 [get_nets {tmds_clock}]
create_generated_clock -name pixel_clock -source [get_nets {tmds_clock}] -master_clock tmds_clock -divide_by 5 [get_nets {hdmi_pixel_clock}]
create_generated_clock -name clock_d -source [get_nets {clock}] -master_clock input_clock -divide_by 1 -multiply_by 1 [get_nets {clock_d}]
//create_generated_clock -name button_clock -source [get_nets {hdmi_pixel_clock}] -master_clock pixel_clock -divide_by 1048576 [get_nets {button_clock}]
