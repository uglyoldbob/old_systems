set_global_assignment -entity "max10_hdmi_output" -library "lib_max10_hdmi_output" -name IP_TOOL_NAME "altera_gpio_lite"
set_global_assignment -entity "max10_hdmi_output" -library "lib_max10_hdmi_output" -name IP_TOOL_VERSION "23.1"
set_global_assignment -entity "max10_hdmi_output" -library "lib_max10_hdmi_output" -name IP_TOOL_ENV "mwpim"
set_global_assignment -library "lib_max10_hdmi_output" -name SPD_FILE [file join $::quartus(sip_path) "max10_hdmi_output.spd"]

set_global_assignment -library "lib_max10_hdmi_output" -name MISC_FILE [file join $::quartus(sip_path) "max10_hdmi_output_sim/max10_hdmi_output.vhd"]
set_global_assignment -library "lib_max10_hdmi_output" -name MISC_FILE [file join $::quartus(sip_path) "max10_hdmi_output_sim/altera_gpio_lite/altera_gpio_lite.sv"]
set_global_assignment -library "lib_max10_hdmi_output" -name MISC_FILE [file join $::quartus(sip_path) "max10_hdmi_output_sim/altera_gpio_lite/mentor/altera_gpio_lite.sv"]
