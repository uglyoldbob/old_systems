set_time_format -unit ns -decimal_places 3

create_clock -name {clock} -period 20.000 -waveform { 0.000 10.000 } [get_ports {clock}]
create_clock -name {clocko} -period 240.000 -waveform { 0.000 120.000 } [get_nets {nes_cpu:cpu|clock_divider:clockd|clocko}]
create_clock -name {c3} -period 240.000 -waveform { 0.000 120.000 } [get_nets {nes_cpu:cpu|clock_divider:clockd|c3}]
create_clock -name {c4} -period 80.000 -waveform { 0.000 40.000 } [get_nets {nes_cpu:cpu|clock_divider:clockd|c4}]
create_clock -name {memory_clock} -period 80.000 -waveform { 0.000 40.000 } [get_nets {memory_clock}]

derive_clock_uncertainty

set_input_delay -clock clock 0 [get_ports {clock}]
set_input_delay -clock clock 20 [get_ports write_address*]
set_input_delay -clock clock 20 [get_ports write_cs*]
set_input_delay -clock clock 20 [get_ports write_rw]
set_input_delay -clock clock 20 [get_ports write_signal]
set_input_delay -clock clock 20 [get_ports write_trigger]
set_input_delay -clock clock 20 [get_ports write_value*]

set_false_path -from [get_ports reset] -to [get_clocks clock]
