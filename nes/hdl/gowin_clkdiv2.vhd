--Copyright (C)2014-2024 Gowin Semiconductor Corporation.
--All rights reserved.
--File Title: IP file
--Tool Version: V1.9.10.03
--Part Number: GW2AR-LV18QN88C8/I7
--Device: GW2AR-18
--Device Version: C
--Created Time: Mon Dec 23 14:58:31 2024

library IEEE;
use IEEE.std_logic_1164.all;

entity Gowin_CLKDIV2 is
    port (
        clkout: out std_logic;
        hclkin: in std_logic;
        resetn: in std_logic
    );
end Gowin_CLKDIV2;

architecture Behavioral of Gowin_CLKDIV2 is

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

end Behavioral; --Gowin_CLKDIV2
