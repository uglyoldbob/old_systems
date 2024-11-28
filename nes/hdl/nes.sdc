set_time_format -unit ns -decimal_places 3

create_clock -name {fast_clock} -period 13.468 -waveform { 0.000 6.734 } [get_ports {fast_clock}]
create_clock -name {clock} -period 40.404 -waveform { 0.000 20.202 } [get_ports {clock}]
create_clock -name {clocko} -period 484.848 -waveform { 0.000 242.424 } [get_nets {nes_cpu:cpu|clock_divider:clockd|clocko}]
create_clock -name {c3} -period 484.848 -waveform { 0.000 242.424 } [get_nets {nes_cpu:cpu|clock_divider:clockd|c3}]
create_clock -name {c4} -period 161.616 -waveform { 0.000 80.808 } [get_nets {nes_cpu:cpu|clock_divider:clockd|c4}]
create_clock -name {memory_clock} -period 161.616 -waveform { 0.000 80.808 } [get_nets {memory_clock}]

derive_clock_uncertainty

set_input_delay -clock clock 0 [get_ports {clock}]
set_input_delay -clock clock 20 [get_ports write_address*]
set_input_delay -clock clock 20 [get_ports write_cs*]
set_input_delay -clock clock 20 [get_ports write_rw]
set_input_delay -clock clock 20 [get_ports write_signal]
set_input_delay -clock clock 20 [get_ports write_trigger]
set_input_delay -clock clock 20 [get_ports write_value*]

set_false_path -from [get_ports reset] -to [get_clocks clock]
