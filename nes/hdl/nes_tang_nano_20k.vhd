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
    signal hdmi_pixel_clock: std_logic;
    signal tmds_clock: std_logic;

    component tmds_pll
        port (
            clkout: out std_logic;
            clkin: in std_logic
        );
    end component;

    component tmds_div
        port (
            clkout: out std_logic;
            hclkin: in std_logic;
            resetn: in std_logic
        );
    end component;

begin
    leds(5 downto 1) <= "10101";

    hdmi_pll: tmds_pll port map(
        clkout => tmds_clock,
        clkin => clock);

    tmds_maker: tmds_div port map (
        clkout => hdmi_pixel_clock,
        hclkin => tmds_clock,
        resetn => '1'
    );

    hdmi_converter: entity work.hdmi generic map(
        t => "mux",
        h => 1280,
        v => 720,
        hblank => 384,
        vblank => 28) port map(
        pixel_clock => hdmi_pixel_clock,
        tmds_clock => tmds_clock,
        d_0_p => hdmi_d_0_p,
        d_0_n => hdmi_d_0_n,
        d_1_p => hdmi_d_1_p,
        d_1_n => hdmi_d_1_n,
        d_2_p => hdmi_d_2_p,
        d_2_n => hdmi_d_2_n,
        ck_p => hdmi_ck_p,
        ck_n => hdmi_ck_n,
        cec => hdmi_cec,
        i2c_scl => hdmi_i2c_scl,
        i2c_sda => hdmi_i2c_sda,
        hpd => hdmi_hpd,
        r => "00000000",
        g => "10101010",
        b => "11001100",
        hsync => '0',
        vsync => '0');

    process (clock)
    begin
        if rising_edge(clock) then
            leds(0) <= not leds(0);
        end if;
    end process;
end Behavioral;

