//Copyright (C)2014-2024 GOWIN Semiconductor Corporation.
//All rights reserved.
//File Title: Timing Constraints file
//Tool Version: V1.9.10.03 
//Created Time: 2024-11-14 12:27:06
create_clock -name input_clock -period 37.037 -waveform {0 18.518} [get_ports {clock}]
create_generated_clock -name tmds_clock -source [get_ports {clock}] -master_clock input_clock -divide_by 4 -multiply_by 55 [get_nets {tmds_clock}]
create_generated_clock -name hdmi_pixel_clock -source [get_nets {tmds_clock}] -master_clock tmds_clock -divide_by 5 [get_ports {hdmi_ck_p}]
