library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity nes_tang_nano_20k is
   Port (
		clock: in std_logic;
        hdmi_d_p: out std_logic_vector(2 downto 0);
        hdmi_d_n: out std_logic_vector(2 downto 0);
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
        test: out std_logic_vector(1 downto 0);
        leds: out std_logic_vector(5 downto 0));
end nes_tang_nano_20k;

architecture Behavioral of nes_tang_nano_20k is
	signal button_clock: std_logic;

	signal rgb: std_logic_vector(23 downto 0);
    signal hdmi_pixel_clock: std_logic;

    signal pll_lock: std_logic;

	signal double_tmds_clock: std_logic;
    signal tmds_clock: std_logic;

    signal tmds10_0: std_logic_vector(9 downto 0);
	signal tmds10_1: std_logic_vector(9 downto 0);
	signal tmds10_2: std_logic_vector(9 downto 0);

	signal tmds_0: std_logic_vector(0 downto 0);
	signal tmds_1: std_logic_vector(0 downto 0);
	signal tmds_2: std_logic_vector(0 downto 0);

    signal tmds_0_post: std_logic_vector(0 downto 0);
    signal tmds_1_post: std_logic_vector(0 downto 0);
    signal tmds_2_post: std_logic_vector(0 downto 0);
    signal tmds_clk_post: std_logic_vector(0 downto 0);
	signal tmds_clk_signal: std_logic;

    signal tmds_0_ddr: std_logic_vector(1 downto 0);
	signal tmds_1_ddr: std_logic_vector(1 downto 0);
	signal tmds_2_ddr: std_logic_vector(1 downto 0);

	signal tmds: std_logic_vector(2 downto 0);

	signal hdmi_row: std_logic_vector(10 downto 0);
	signal hdmi_column: std_logic_vector(11 downto 0);
	signal hdmi_hstart: std_logic;
	signal hdmi_vstart: std_logic;
	signal hdmi_pvalid: std_logic;

	signal crosshair_row: std_logic_vector(9 downto 0) := std_logic_vector(to_unsigned(80, 10));
	signal crosshair_column: std_logic_vector(10 downto 0) := std_logic_vector(to_unsigned(5, 11));

	signal debounce_buttona: std_logic;
	signal debounce_buttonb: std_logic;

	signal nes_clock: std_logic;
	signal nes_reset: std_logic;
	signal nes_oe: std_logic_vector(1 downto 0);
	signal nes_address: std_logic_vector(15 downto 0);

	signal ppu_r: std_logic_vector(7 downto 0);
	signal ppu_g: std_logic_vector(7 downto 0);
	signal ppu_b: std_logic_vector(7 downto 0);

	signal write_signal: std_logic;
	signal write_address: std_logic_vector(19 downto 0);
	signal write_value: std_logic_vector(7 downto 0);
	signal write_trigger: std_logic;
	signal write_rw: std_logic;
	signal write_cs: std_logic_vector(1 downto 0);

    component tmds_pll
		port (
			clkout: out std_logic;
			lock: out std_logic;
			clkoutd: out std_logic;
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

	component gowin_clkdiv2
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

	component test_hdmi_out
		port (
			tmds: in std_logic_vector(2 downto 0);
			tmds_clk: in std_logic;
			tmds_clk_n: out std_logic;
			tmds_clk_p: out std_logic;
			tmds_d_n: out std_logic_vector(2 downto 0);
			tmds_d_p: out std_logic_vector(2 downto 0));
	end component;

begin
    leds(5 downto 2) <= "1010";
    leds(1) <= not pll_lock;
    leds(0) <= not hdmi_hpd;

	tmds <= tmds_2 & tmds_1 & tmds_0;
	tmds_clk_signal <= tmds_clk_post(0);

	tmds_buf: test_hdmi_out port map (
		tmds_clk => tmds_clk_signal,
		tmds => tmds,
		tmds_clk_p => hdmi_ck_p,
		tmds_clk_n => hdmi_ck_n,
		tmds_d_p => hdmi_d_p,
		tmds_d_n => hdmi_d_n);

	hdmi_serclk: Gowin_DDR
        port map (
            din => "1111100000",
            fclk => tmds_clock,
            pclk => hdmi_pixel_clock,
            reset => '0',
            q => tmds_clk_post);

    hdmi_ser0: Gowin_DDR
        port map (
            din => tmds10_0,
            fclk => tmds_clock,
            pclk => hdmi_pixel_clock,
            reset => '0',
            q => tmds_0);
    hdmi_ser1: Gowin_DDR
        port map (
            din => tmds10_1,
            fclk => tmds_clock,
            pclk => hdmi_pixel_clock,
            reset => '0',
            q => tmds_1);
    hdmi_ser2: Gowin_DDR
        port map (
            din => tmds10_2,
            fclk => tmds_clock,
            pclk => hdmi_pixel_clock,
            reset => '0',
            q => tmds_2);

    hdmi_pll: tmds_pll port map(
        lock => pll_lock,
        clkout => double_tmds_clock,
		clkoutd => nes_clock,
        clkin => clock);

	pixel_div: gowin_clkdiv2 port map(
		hclkin => double_tmds_clock,
		clkout => tmds_clock,
		resetn => '1');
    
    tmds_maker: tmds_div port map (
        clkout => hdmi_pixel_clock,
        hclkin => tmds_clock,
        resetn => '1'
    );

    hdmi_converter: entity work.hdmi2 generic map(
			hsync_polarity => '1',
			vsync_polarity => '1',
			h => 1280,
			v => 720,
			hblank_width => 370,
			hsync_porch => 220,
			hsync_width => 40,
			vblank_width => 30,
			vsync_porch => 20,
			vsync_width => 5) port map(
			reset => '0',
			pixel_clock => hdmi_pixel_clock,
			tmds_clock => tmds_clock,
			tmds_0 => tmds10_0,
			tmds_1 => tmds10_1,
			tmds_2 => tmds10_2,
			cec => hdmi_cec,
			i2c_scl => hdmi_i2c_scl,
			i2c_sda => hdmi_i2c_sda,
			hpd => hdmi_hpd,
			row_out => hdmi_row,
			column_out => hdmi_column,
			hstart => hdmi_hstart,
			vstart => hdmi_vstart,
			pvalid => hdmi_pvalid,
			test => test,
			r => rgb(23 downto 16),
			g => rgb(15 downto 8),
			b => rgb(7 downto 0));

	process (hdmi_pixel_clock)
	begin
		if rising_edge(hdmi_pixel_clock) then
			if hdmi_column < std_logic_vector(to_signed(256, 12)) or hdmi_column > std_logic_vector(to_signed(1024, 12)) then
				rgb <= (others => '0');
			else
				rgb <= ppu_r & ppu_g & ppu_b;
			end if;
		end if;
	end process;

	bc: entity work.large_divider generic map(bits => 20) port map(clock => hdmi_pixel_clock, ckout => button_clock);

	btn1: entity work.switch_debounce port map(
		slowclock => button_clock,
		clock => hdmi_pixel_clock,
		din => buttons(0),
		dout => debounce_buttona);

	btn2: entity work.switch_debounce port map(
		slowclock => button_clock,
		clock => hdmi_pixel_clock,
		din => buttons(1),
		dout => debounce_buttonb);

	write_signal <= '0';
	write_trigger <= '0';
	nes_reset <= '0';

	nes: entity work.nes generic map(
		random_noise => '1') port map (
		ppu_r => ppu_r,
		ppu_g => ppu_g,
		ppu_b => ppu_b,
		write_signal => write_signal,
		write_address => write_address,
		write_value => write_value,
		write_trigger => write_trigger,
		write_rw => write_rw,
		write_cs => write_cs,
		reset => nes_reset,
		cpu_oe => nes_oe,
		cpu_memory_address => nes_address,
		fast_clock => hdmi_pixel_clock,
		clock => nes_clock,
		hdmi_vsync => hdmi_vstart);
end Behavioral;

