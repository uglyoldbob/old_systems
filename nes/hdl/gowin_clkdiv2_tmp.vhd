--Copyright (C)2014-2024 Gowin Semiconductor Corporation.
--All rights reserved.
--File Title: Template file for instantiation
--Tool Version: V1.9.9.03 Education
--Part Number: GW2AR-LV18QN88C8/I7
--Device: GW2AR-18
--Device Version: C
--Created Time: Mon Nov 25 10:16:43 2024

--Change the instance name and port connections to the signal names
----------Copy here to design--------

component gowin_clkdiv2
    port (
        clkout: out std_logic;
        hclkin: in std_logic;
        resetn: in std_logic
    );
end component;

your_instance_name: gowin_clkdiv2
    port map (
        clkout => clkout,
        hclkin => hclkin,
        resetn => resetn
    );

----------Copy end-------------------
