library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity nes_tang_nano_20k is
   Port (
		clock: in std_logic;
        hdmi_d_0: out std_logic;
        hdmi_d_1: out std_logic;
        hdmi_d_2: out std_logic;
        hdmi_ck: out std_logic;
        hdmi_cec: inout std_logic;
        hdmi_i2c_scl: inout std_logic;
        hdmi_i2c_sda: inout std_logic;
        hdmi_hpd: inout std_logic;
        sd_d: inout std_logic_vector(3 downto 0);
        sd_ck: out std_logic;
        sd_cmd: out std_logic;
        buttons: in std_logic_vector(1 downto 0);
        test: out std_logic;
        leds: out std_logic_vector(5 downto 0));
end nes_tang_nano_20k;

architecture Behavioral of nes_tang_nano_20k is
    signal random_data: std_logic_vector(31 downto 0);
    signal hdmi_pixel_clock: std_logic;

    signal divider: std_logic_vector(7 downto 0);
    signal divider2: std_logic_vector(7 downto 0);
    signal divider3: std_logic_vector(7 downto 0);

    signal pll_lock: std_logic;

    signal tmds_clock: std_logic;

    signal tmds_0: std_logic_vector(9 downto 0);
	signal tmds_1: std_logic_vector(9 downto 0);
	signal tmds_2: std_logic_vector(9 downto 0);
    signal tmds_test: std_logic_vector(9 downto 0);

    signal tmds_0_post: std_logic_vector(0 downto 0);
    signal tmds_1_post: std_logic_vector(0 downto 0);
    signal tmds_2_post: std_logic_vector(0 downto 0);
    signal tmds_clk_post: std_logic_vector(0 downto 0);

    signal tmds_0_ddr: std_logic_vector(1 downto 0);
	signal tmds_1_ddr: std_logic_vector(1 downto 0);
	signal tmds_2_ddr: std_logic_vector(1 downto 0);

    component tmds_pll
        port (
            clkout: out std_logic;
            lock: out std_logic;
            clkin: in std_logic
        );
    end component;

    component tiny_hdmi_pll
        port (
            clkout: out std_logic;
            clkin: in std_logic
        );
    end component;

    component slow_pll
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

    component Gowin_DDR
        port (
            din: in std_logic_vector(9 downto 0);
            fclk: in std_logic;
            pclk: in std_logic;
            reset: in std_logic;
            q: out std_logic_vector(0 downto 0)
        );
    end component;  

begin
    leds(5 downto 3) <= "101";
    test <= '1';
    leds(2) <= divider3(7);
    leds(1) <= not pll_lock;
    leds(0) <= not hdmi_hpd;

    hdmi_d_0 <= tmds_0_post(0);
    hdmi_d_1 <= tmds_1_post(0);
    hdmi_d_2 <= tmds_2_post(0);
    hdmi_ck <= tmds_clk_post(0);

    hdmi_clock_ser: Gowin_DDR
        port map (
            din => "1111100000",
            fclk => tmds_clock,
            pclk => hdmi_pixel_clock,
            reset => '0',
            q => tmds_clk_post);

    hdmi_ser0: Gowin_DDR
        port map (
            din => tmds_0,
            fclk => tmds_clock,
            pclk => hdmi_pixel_clock,
            reset => '0',
            q => tmds_0_post);
    hdmi_ser1: Gowin_DDR
        port map (
            din => tmds_1,
            fclk => tmds_clock,
            pclk => hdmi_pixel_clock,
            reset => '0',
            q => tmds_1_post);
    hdmi_ser2: Gowin_DDR
        port map (
            din => tmds_2,
            fclk => tmds_clock,
            pclk => hdmi_pixel_clock,
            reset => '0',
            q => tmds_2_post);
--    d0_mux: entity work.tmds_multiplexer port map(
--		reset => '0',
--		clock => tmds_clock,
--		pixel_clock => hdmi_pixel_clock,
--		din => tmds_test,
--		dout => tmds_0_ddr
--	);
--	
--	d0_output: entity work.ddr generic map(t => "mux")
--		port map(
--			din => tmds_0_ddr,
--			dout => hdmi_d_0,
--			clock => tmds_clock
--	);

    process (tmds_clock)
    begin
        if rising_edge(tmds_clock) then
            divider <= std_logic_vector(unsigned(divider) + 1);
        end if;
    end process;

    process (divider(7))
    begin
        if rising_edge(divider(7)) then
            divider2 <= std_logic_vector(unsigned(divider2) + 1);
        end if;
    end process;

    process (divider2(7))
    begin
        if rising_edge(divider2(7)) then
            divider3 <= std_logic_vector(unsigned(divider3) + 1);
        end if;
    end process;

    process (divider3(7))
    begin
        if rising_edge(divider3(7)) then
            case tmds_test is
                when "0000000001" => tmds_test <= "0000000010";
                when "0000000010" => tmds_test <= "0000000100";
                when "0000000100" => tmds_test <= "0000001000";
                when "0000001000" => tmds_test <= "0000010000";
                when "0000010000" => tmds_test <= "0000000001";
                when "0000100000" => tmds_test <= "0001000000";
                when "0001000000" => tmds_test <= "0010000000";
                when "0010000000" => tmds_test <= "0100000000";
                when "0100000000" => tmds_test <= "1000000000";
                when "1000000000" => tmds_test <= "0000000001";
                when others => tmds_test <= "0000000001";
            end case;
        end if;
    end process;

--    hdmi_pll: tmds_pll port map(
--        lock => pll_lock,
--        clkout => tmds_clock,
--        clkin => clock);

    tiny_hdmi: tiny_hdmi_pll
        port map (
            clkout => tmds_clock,
            clkin => clock
        );

--    slow_pll_i: slow_pll
--        port map (
--            clkout => hdmi_pixel_clock,
--            clkin => clock);
--    tmds_clock <= clock;
    
    tmds_maker: tmds_div port map (
        clkout => hdmi_pixel_clock,
        hclkin => tmds_clock,
        resetn => '1'
    );

    hdmi_converter: entity work.hdmi generic map(
        t => "mux",
        h => 800,
		v => 600,
		hblank_width => 256,
		hsync_porch => 40,
		hsync_width => 128,
		vblank_width => 28,
		vsync_porch => 1,
		vsync_width => 4) port map(
        reset => not pll_lock,
        pixel_clock => hdmi_pixel_clock,
        tmds_clock => tmds_clock,
        tmds_0 => tmds_0,
        tmds_1 => tmds_1,
        tmds_2 => tmds_2,
        cec => hdmi_cec,
        i2c_scl => hdmi_i2c_scl,
        i2c_sda => hdmi_i2c_sda,
        hpd => hdmi_hpd,
        r => random_data(23 downto 16),
        g => random_data(15 downto 8),
        b => random_data(7 downto 0));

    random: entity work.lfsr32 port map(
		clock => hdmi_pixel_clock,
		dout => random_data);
end Behavioral;

