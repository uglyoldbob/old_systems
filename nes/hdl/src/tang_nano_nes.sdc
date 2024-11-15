//Copyright (C)2014-2024 GOWIN Semiconductor Corporation.
//All rights reserved.
//File Title: Timing Constraints file
//Tool Version: V1.9.10.03 
//Created Time: 2024-11-15 09:02:54
create_clock -name input_clock -period 37.037 -waveform {0 18.518} [get_ports {clock}]
//create_generated_clock -name tmds_clock -source [get_nets {clock}] -master_clock input_clock -divide_by 4 -multiply_by 55 [get_nets {hdmi_ck_p}]
create_generated_clock -name clock_d -source [get_nets {clock}] -master_clock input_clock -divide_by 1 -multiply_by 1 [get_nets {clock_d}]
