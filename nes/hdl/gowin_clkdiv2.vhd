--Copyright (C)2014-2024 Gowin Semiconductor Corporation.
--All rights reserved.
--File Title: IP file
--Tool Version: V1.9.9.03 Education
--Part Number: GW2AR-LV18QN88C8/I7
--Device: GW2AR-18
--Device Version: C
--Created Time: Mon Nov 25 10:16:43 2024

library IEEE;
use IEEE.std_logic_1164.all;

entity gowin_clkdiv2 is
    port (
        clkout: out std_logic;
        hclkin: in std_logic;
        resetn: in std_logic
    );
end gowin_clkdiv2;

architecture Behavioral of gowin_clkdiv2 is

    --component declaration
    component CLKDIV2
        port (
            CLKOUT: out std_logic;
            HCLKIN: in std_logic;
            RESETN: in std_logic
        );
    end component;

begin
    clkdiv2_inst: CLKDIV2
        port map (
            CLKOUT => clkout,
            HCLKIN => hclkin,
            RESETN => resetn
        );

end Behavioral; --gowin_clkdiv2
