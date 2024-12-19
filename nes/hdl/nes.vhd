library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity nes is
	Generic (
		sim: in std_logic := '0';
		ramtype: string := "wishbone";
        rambits: integer := 3;
		random_noise: in std_logic := '1';
		unified_ram: std_logic := '0');
   Port (
		ignore_sync: in std_logic := '0';
		hdmi_pixel_out: out std_logic_vector(23 downto 0);
		hdmi_vsync: in std_logic;
		hdmi_row: in std_logic_vector(10 downto 0);
		hdmi_column: in std_logic_vector(11 downto 0);
		hdmi_valid_out: out std_logic;
		hdmi_pvalid: in std_logic;
		hdmi_line_done: out std_logic;
		hdmi_line_ready: in std_logic;

		testo: out std_logic;

		rom_wb_ack: in std_logic;
		rom_wb_d_miso: in std_logic_vector(2**rambits-1 downto 0);
		rom_wb_d_mosi: out std_logic_vector(2**rambits-1 downto 0);
		rom_wb_err: in std_logic;
		rom_wb_addr: out std_logic_vector(25-rambits downto 0);
		rom_wb_bte: out std_logic_vector(1 downto 0);
		rom_wb_cti: out std_logic_vector(2 downto 0);
		rom_wb_cyc: out std_logic;
		rom_wb_sel: out std_logic_vector(rambits-3 downto 0);
		rom_wb_stb: out std_logic;
		rom_wb_we: out std_logic;
		
		d_a: out std_logic_vector(7 downto 0) := x"00";
		d_x: out std_logic_vector(7 downto 0) := x"00";
		d_y: out std_logic_vector(7 downto 0) := x"00";
		d_pc: out std_logic_vector(15 downto 0);
		d_sp: out std_logic_vector(7 downto 0) := x"fd";
		d_flags: out std_logic_vector(7 downto 0) := x"24";
		d_memory_clock: out std_logic;
		d_subcycle: out std_logic_vector(3 downto 0);
		d_cycle: out std_logic_vector(14 downto 0);
		instruction_toggle_out: out std_logic;
		reset: in std_logic;
		clock: in std_logic;
		cpu_oe: out std_logic_vector(1 downto 0);
		cpu_memory_address: out std_logic_vector(15 downto 0);
	   whocares: out std_logic;
		cs_out: out std_logic_vector(3 downto 0);
		otherstuff: out std_logic_vector(15 downto 0));
end nes;

architecture Behavioral of nes is
	signal cpu_address: std_logic_vector(15 downto 0);
	signal cpu_dout: std_logic_vector(7 downto 0);
	signal cpu_din: std_logic_vector(7 downto 0);
	signal cpu_din_ready: std_logic;
	signal cpu_dready: std_logic;
	signal cpu_rw: std_logic;
	signal cpu_memory_clock: std_logic;
	signal cpu_memory_start: std_logic;
	signal memory_clock: std_logic;
	
	signal cpu_sram_din: std_logic_vector(7 downto 0);
	signal cpu_sram_dout: std_logic_vector(7 downto 0);
	signal cpu_ram_cs: std_logic;
	
	signal cpu_ppu_cs: std_logic;
	signal cpu_ppu_din: std_logic_vector(7 downto 0);
	signal cpu_ppu_dout: std_logic_vector(7 downto 0);
	
	signal ppu_int: std_logic;
	signal ppu_ale: std_logic;
	signal ppu_ad: std_logic_vector(7 downto 0);
	signal ppu_a: std_logic_vector(5 downto 0);
	signal ppu_din: std_logic_vector(7 downto 0);
	signal ppu_rd: std_logic;
	signal ppu_wr: std_logic;
	signal ppu_clock: std_logic;
	
	signal ppu_pixel_valid: std_logic;
	signal ppu_hstart: std_logic;
	signal ppu_vstart: std_logic;
	signal ppu_row: std_logic_vector(8 downto 0);
	signal ppu_column: std_logic_vector(8 downto 0);
	
	signal cpu_apu_cs: std_logic;
	
	signal cpu_cartridge_cs: std_logic;
	signal cpu_cartridge_din: std_logic_vector(7 downto 0);
	signal cpu_cartridge_din_ready: std_logic;

	signal pause: std_logic;
	signal fsync_pause: std_logic;
	signal hdmi_vsync_delay: std_logic;
	signal hdmi_vsync_trigger: std_logic;
	signal ppu_vsync_sync: std_logic;
	
	signal ppu_pixel: std_logic_vector(23 downto 0);
	
	signal reset_sync: std_logic;
	signal reset_chain: std_logic;
begin
	whocares <= clock;
	otherstuff <= cpu_address;
	cpu_memory_address <= cpu_address;
	cs_out <= cpu_ram_cs & cpu_ppu_cs & cpu_apu_cs & cpu_cartridge_cs;
	
	d_memory_clock <= memory_clock;
	pause <= (cpu_memory_clock and not cpu_din_ready) or (fsync_pause and not ignore_sync);

	testo <= pause;

	process (clock)
	begin
		if rising_edge(clock) then
			reset_sync <= reset_chain;
			reset_chain <= reset;
		end if;
	end process;
	
	process (all)
	begin
		if cpu_address(15 downto 13) = "000" then
			cpu_ram_cs <= '1';
		else
			cpu_ram_cs <= '0';
		end if;
		if not cpu_address(15) and not cpu_address(14) and (cpu_address(13) or cpu_address(12)) then
			cpu_ppu_cs <= '1';
		else
			cpu_ppu_cs <= '0';
		end if;
		if cpu_address(15 downto 5) = "00000000000" then
			cpu_apu_cs <= '1';
		else
			cpu_apu_cs <= '0';
		end if;
		cpu_cartridge_cs <= not (cpu_ram_cs or cpu_ppu_cs or cpu_apu_cs);
		if cpu_ram_cs then
			cpu_din <= cpu_sram_dout;
		elsif cpu_ppu_cs then
			cpu_din <= cpu_ppu_din;
		elsif cpu_cartridge_cs then
			cpu_din <= cpu_cartridge_din;
		else
			cpu_din <= "00000000";
		end if;
		if cpu_cartridge_cs then
			cpu_din_ready <= cpu_cartridge_din_ready;
		else
			cpu_din_ready <= '1';
		end if;
	end process;
	
	process (clock)
	begin
		if rising_edge(clock) then
			memory_clock <= cpu_memory_clock;
		end if;
	end process;
	
	process (reset_sync, memory_clock)
	begin
		if reset_sync = '1' then
			cpu_dready <= '0';
		elsif rising_edge(memory_clock) then
			if cpu_ram_cs or cpu_cartridge_cs then
				cpu_dready <= '1';
			end if;
		end if;
	end process;
	
	cpu_ram: entity work.clocked_sram generic map (
		bits => 11
	) port map (
		clock => memory_clock,
		cs => cpu_ram_cs,
		address => cpu_address(10 downto 0),
		rw => cpu_rw,
		din => cpu_dout,
		dout => cpu_sram_dout
	);
	
	cpu: entity work.nes_cpu generic map(
		ramtype => ramtype) port map (
		pause_cpu => pause,
		d_a => d_a,
		d_x => d_x,
		d_y => d_y,
		d_pc => d_pc,
		d_sp => d_sp,
		d_flags => d_flags,
		d_subcycle => d_subcycle,
		d_cycle => d_cycle,
		instruction_toggle_out => instruction_toggle_out,
		clock => clock,
		ppu_clock => ppu_clock,
		memory_clock => cpu_memory_clock,
		memory_start => cpu_memory_start,
		memory_cycle_done => cpu_dready,
		rw => cpu_rw,
		oe => cpu_oe,
		reset => reset_sync,
		din => cpu_din,
		dout => cpu_dout,
		nmi => '1',
		irq => '1',
		tst => '0',
		address => cpu_address);
	
	ppu: entity work.nes_ppu generic map(
		sim => sim,
		random_noise => random_noise,
		ramtype => ramtype) port map (
		r_out => ppu_pixel(23 downto 16),
		g_out => ppu_pixel(15 downto 8),
		b_out => ppu_pixel(7 downto 0),
		pixel_valid => ppu_pixel_valid,
		hstart => ppu_hstart,
		vstart => ppu_vstart,
		row => ppu_row,
		column => ppu_column,
		clock => ppu_clock,
		reset => reset_sync,
		cpu_addr => cpu_address(2 downto 0),
		cpu_cs => cpu_ppu_cs,
		cpu_rw => cpu_rw,
		cpu_dout => cpu_dout,
		cpu_din => cpu_ppu_din,
		cpu_mem_clock => memory_clock,
		int => ppu_int,
		ppu_ale => ppu_ale,
		ppu_ad => ppu_ad,
		ppu_a => ppu_a,
		ppu_din => ppu_din,
		ppu_rd => ppu_rd,
		ppu_wr => ppu_wr);

	ppu3: entity work.nes_tripler port map(
		hdmi_line_ready => hdmi_line_ready,
		hdmi_pvalid => hdmi_pvalid,
		hdmi_column => hdmi_column,
		hdmi_row => hdmi_row,
		hdmi_vsync => hdmi_vsync,
		hdmi_pixel_out => hdmi_pixel_out,
		hdmi_valid_out => hdmi_valid_out,
		hdmi_line_done => hdmi_line_done,
		ppu_clock => ppu_clock,
		ppu_pixel => ppu_pixel,
		ppu_pixel_valid => ppu_pixel_valid,
		ppu_hstart => ppu_hstart,
		ppu_vstart => ppu_vstart,
		ppu_row => ppu_row,
		ppu_column => ppu_column,
		fsync_pause => fsync_pause,
		clock => clock);
	
	cartridge: entity work.nes_cartridge generic map(
		ramtype => ramtype,
        rambits => rambits,
		unified_ram => unified_ram) port map (
		rom_wb_ack => rom_wb_ack,
		rom_wb_d_miso => rom_wb_d_miso,
		rom_wb_d_mosi => rom_wb_d_mosi,
		rom_wb_err => rom_wb_err,
		rom_wb_addr => rom_wb_addr,
		rom_wb_bte => rom_wb_bte,
		rom_wb_cti => rom_wb_cti,
		rom_wb_cyc => rom_wb_cyc,
		rom_wb_sel => rom_wb_sel,
		rom_wb_stb => rom_wb_stb,
		rom_wb_we => rom_wb_we,
		cpu_data_out => cpu_dout,
		cpu_data_in => cpu_cartridge_din,
		cpu_data_in_ready => cpu_cartridge_din_ready,
		cpu_addr => cpu_address,
		cpu_memory_start => cpu_memory_start,
		ppu_data_in => "00000000",
		ppu_addr => "00000000000000",
		ppu_addr_a13_n => '1',
		ppu_wr => '0',
		ppu_rd => '0',
		cpu_rw => cpu_rw,
		romsel => cpu_address(15),
		m2 => memory_clock,
		clock => clock);

end Behavioral;

