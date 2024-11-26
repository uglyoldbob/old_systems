--Copyright (C)2014-2024 Gowin Semiconductor Corporation.
--All rights reserved.
--File Title: Template file for instantiation
--Tool Version: V1.9.9.03 Education
--Part Number: GW2AR-LV18QN88C8/I7
--Device: GW2AR-18
--Device Version: C
--Created Time: Tue Nov 26 12:25:40 2024

--Change the instance name and port connections to the signal names
----------Copy here to design--------

component gowin_nes_pll
    port (
        clkout: out std_logic;
        lock: out std_logic;
        clkoutd: out std_logic;
        clkin: in std_logic
    );
end component;

your_instance_name: gowin_nes_pll
    port map (
        clkout => clkout,
        lock => lock,
        clkoutd => clkoutd,
        clkin => clkin
    );

----------Copy end-------------------
