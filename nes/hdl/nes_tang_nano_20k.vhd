library IEEE;
use IEEE.STD_LOGIC_1164.ALL;

entity nes_tang_nano_20k is
   Port (
		clock: in std_logic;
        hdmi_d_0_p: out std_logic;
        hdmi_d_0_n: out std_logic;
        hdmi_d_1_p: out std_logic;
        hdmi_d_1_n: out std_logic;
        hdmi_d_2_p: out std_logic;
        hdmi_d_2_n: out std_logic;
        hdmi_ck_p: out std_logic;
        hdmi_ck_n: out std_logic;
        hdmi_cec: inout std_logic;
        hdmi_i2c_scl: inout std_logic;
        hdmi_i2c_sda: inout std_logic;
        hdmi_hpd: inout std_logic;
        sd_d: inout std_logic_vector(3 downto 0);
        sd_ck: out std_logic;
        sd_cmd: out std_logic;
        buttons: in std_logic_vector(1 downto 0);
        leds: out std_logic_vector(5 downto 0));
end nes_tang_nano_20k;

architecture Behavioral of nes_tang_nano_20k is

begin
    leds(5 downto 1) <= "10101";

    process (clock)
    begin
        if rising_edge(clock) then
            leds(0) <= not leds(0);
        end if;
    end process;
end Behavioral;

