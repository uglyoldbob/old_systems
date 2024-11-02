set_time_format -unit ns -decimal_places 3

create_clock -name {clock} -period 20.000 -waveform { 0.000 10.000 } [get_ports {clock}]
create_clock -name {clocko} -period 240.000 -waveform { 0.000 120.000 } [get_nets {nes_cpu:cpu|clock_divider:clockd|clocko}]
create_clock -name {clocko2} -period 240.000 -waveform { 0.000 120.000 } [get_nets {nes_cpu:cpu|clock_divider:clockd|clocko2}]
create_clock -name {c3} -period 240.000 -waveform { 0.000 120.000 } [get_nets {nes_cpu:cpu|clock_divider:clockd|c3}]
