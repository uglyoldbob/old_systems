library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity nes is
	Generic (
        FREQ: integer := 74250000;
		clockbuf: string;
		sim: in std_logic := '0';
        softcpu: std_logic := '1';
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

        uart_tx: out std_logic;
        uart_rx: in std_logic := '1';
		
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
		soft_cpu_clock: in std_logic;
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

    component VexRiscv
        port (
          externalResetVector: in std_logic_vector(31 downto 0);
          timerInterrupt: in std_logic;
          softwareInterrupt: in std_logic;
          externalInterruptArray: in std_logic_vector(31 downto 0);
          iBusWishbone_CYC: out std_logic;
          iBusWishbone_STB: out std_logic;
          iBusWishbone_ACK: in std_logic;
          iBusWishbone_WE: out std_logic;
          iBusWishbone_ADR: out std_logic_vector(29 downto 0);
          iBusWishbone_DAT_MISO: in std_logic_vector(31 downto 0);
          iBusWishbone_DAT_MOSI: out std_logic_vector(31 downto 0);
          iBusWishbone_SEL: out std_logic_vector(3 downto 0);
          iBusWishbone_ERR: in std_logic;
          iBusWishbone_BTE: out std_logic_vector(1 downto 0);
          iBusWishbone_CTI: out std_logic_vector(2 downto 0);
          dBusWishbone_CYC: out std_logic;
          dBusWishbone_STB: out std_logic;
          dBusWishbone_ACK: in std_logic;
          dBusWishbone_WE: out std_logic;
          dBusWishbone_ADR: out std_logic_vector(29 downto 0);
          dBusWishbone_DAT_MISO: in std_logic_vector(31 downto 0);
          dBusWishbone_DAT_MOSI: out std_logic_vector(31 downto 0);
          dBusWishbone_SEL: out std_logic_vector(3 downto 0);
          dBusWishbone_ERR: in std_logic;
          dBusWishbone_BTE: out std_logic_vector(1 downto 0);
          dBusWishbone_CTI: out std_logic_vector(2 downto 0);
          clk: in std_logic;
          reset: in std_logic);
    end component;

    signal timerInterrupt: std_logic;
    signal softwareInterrupt: std_logic;
    signal externalInterruptArray: std_logic_vector(31 downto 0);
    signal iBusWishbone_CYC: std_logic;
    signal iBusWishbone_STB: std_logic;
    signal iBusWishbone_ACK: std_logic;
    signal iBusWishbone_WE: std_logic;
    signal iBusWishbone_ADR: std_logic_vector(29 downto 0);
    signal iBusWishbone_DAT_MISO: std_logic_vector(31 downto 0);
    signal iBusWishbone_DAT_MOSI: std_logic_vector(31 downto 0);
    signal iBusWishbone_SEL: std_logic_vector(3 downto 0);
    signal iBusWishbone_ERR: std_logic;
    signal iBusWishbone_BTE: std_logic_vector(1 downto 0);
    signal iBusWishbone_CTI: std_logic_vector(2 downto 0);
    signal dBusWishbone_CYC: std_logic;
    signal dBusWishbone_STB: std_logic;
    signal dBusWishbone_ACK: std_logic;
    signal dBusWishbone_WE: std_logic;
    signal dBusWishbone_ADR: std_logic_vector(29 downto 0);
    signal dBusWishbone_DAT_MISO: std_logic_vector(31 downto 0);
    signal dBusWishbone_DAT_MOSI: std_logic_vector(31 downto 0);
    signal dBusWishbone_SEL: std_logic_vector(3 downto 0);
    signal dBusWishbone_ERR: std_logic;
    signal dBusWishbone_BTE: std_logic_vector(1 downto 0);
    signal dBusWishbone_CTI: std_logic_vector(2 downto 0);

    signal Wishbone_CYC: std_logic;
    signal Wishbone_STB: std_logic;
    signal Wishbone_ACK: std_logic;
    signal Wishbone_WE: std_logic;
    signal Wishbone_ADR: std_logic_vector(29 downto 0);
    signal Wishbone_DAT_MISO: std_logic_vector(31 downto 0);
    signal Wishbone_DAT_MOSI: std_logic_vector(31 downto 0);
    signal Wishbone_SEL: std_logic_vector(3 downto 0);
    signal Wishbone_ERR: std_logic;
    signal Wishbone_BTE: std_logic_vector(1 downto 0);
    signal Wishbone_CTI: std_logic_vector(2 downto 0);

    signal uart_sel: std_logic;
    signal uart_wb_ack: std_logic;
    signal uart_wb_d_miso: std_logic_vector(31 downto 0);
    signal uart_wb_err: std_logic;
    signal uart_wb_we: std_logic;
	signal uart_wb_cyc: std_logic;
	signal uart_wb_stb: std_logic;

    signal bios_sel: std_logic;
    signal bios_data: std_logic_vector(31 downto 0);
    signal bios_data_valid: std_logic;
    signal bios_wb_ack: std_logic;

    signal cpu_only_reset: std_logic := '0';
    signal cpu_reset: std_logic := '0';

	signal por: std_logic := '1';
	signal por_calc: integer range 0 to 1023 := 0;
begin
	whocares <= clock;
	otherstuff <= cpu_address;
	cpu_memory_address <= cpu_address;
	cs_out <= cpu_ram_cs & cpu_ppu_cs & cpu_apu_cs & cpu_cartridge_cs;
	
	d_memory_clock <= memory_clock;
	pause <= (cpu_memory_clock and not cpu_din_ready) or (fsync_pause and not ignore_sync);

    cpu_reset <= cpu_only_reset or reset_sync;

	testo <= pause;

	process (all)
	begin
		if por_calc = 1023 then
			por <= '0';
		else
			por <= '1';
		end if;
	end process;

	process (clock)
	begin
		if rising_edge(clock) then
			if por_calc < 1023 then
				por_calc <= por_calc + 1;
			end if;
			reset_sync <= reset_chain or por;
			reset_chain <= reset;
		end if;
	end process;

    cpugen: if softcpu = '1' generate
        bios: entity work.clocked_sram_init generic map(dbits => 32, bits => 5, filename => "riscv-bios-rust/combios.dat") port map (
            clock => clock, 
            cs => bios_sel, 
            address => Wishbone_ADR(4 downto 0),
            rw => '1',
            din => (others => '0'),
            dout => bios_data,
            dout_valid => bios_wb_ack);

		timerInterrupt <= '0';
		softwareInterrupt <= '0';
		externalInterruptArray <= (others => '0');

        softcpu: VexRiscv port map(
            externalResetVector => x"00010000",
            timerInterrupt => timerInterrupt,
            softwareInterrupt => softwareInterrupt,
            externalInterruptArray => externalInterruptArray,
            iBusWishbone_CYC => iBusWishbone_CYC,
            iBusWishbone_STB => iBusWishbone_STB,
            iBusWishbone_ACK => iBusWishbone_ACK,
            iBusWishbone_WE => iBusWishbone_WE,
            iBusWishbone_ADR => iBusWishbone_ADR,
            iBusWishbone_DAT_MISO => iBusWishbone_DAT_MISO,
            iBusWishbone_DAT_MOSI => iBusWishbone_DAT_MOSI,
            iBusWishbone_SEL => iBusWishbone_SEL,
            iBusWishbone_ERR => iBusWishbone_ERR,
            iBusWishbone_BTE => iBusWishbone_BTE,
            iBusWishbone_CTI => iBusWishbone_CTI,
            dBusWishbone_CYC => dBusWishbone_CYC,
            dBusWishbone_STB => dBusWishbone_STB,
            dBusWishbone_ACK => dBusWishbone_ACK,
            dBusWishbone_WE => dBusWishbone_WE,
            dBusWishbone_ADR => dBusWishbone_ADR,
            dBusWishbone_DAT_MISO => dBusWishbone_DAT_MISO,
            dBusWishbone_DAT_MOSI => dBusWishbone_DAT_MOSI,
            dBusWishbone_SEL => dBusWishbone_SEL,
            dBusWishbone_ERR => dBusWishbone_ERR,
            dBusWishbone_BTE => dBusWishbone_BTE,
            dBusWishbone_CTI => dBusWishbone_CTI,
            clk => soft_cpu_clock,
            reset => cpu_reset);

        wbc: entity work.wishbone_host_combiner generic map(sim => sim) port map(
            wba_cyc => iBusWishbone_CYC,
            wba_stb => iBusWishbone_STB,
            wba_ack => iBusWishbone_ACK,
            wba_we => iBusWishbone_WE,
            wba_addr => iBusWishbone_ADR,
            wba_d_miso => iBusWishbone_DAT_MISO,
            wba_d_mosi => iBusWishbone_DAT_MOSI,
            wba_sel => iBusWishbone_SEL,
            wba_err => iBusWishbone_ERR,
            wba_bte => iBusWishbone_BTE,
            wba_cti => iBusWishbone_CTI,
            wbb_cyc => dBusWishbone_CYC,
            wbb_stb => dBusWishbone_STB,
            wbb_ack => dBusWishbone_ACK,
            wbb_we => dBusWishbone_WE,
            wbb_addr => dBusWishbone_ADR,
            wbb_d_miso => dBusWishbone_DAT_MISO,
            wbb_d_mosi => dBusWishbone_DAT_MOSI,
            wbb_sel => dBusWishbone_SEL,
            wbb_err => dBusWishbone_ERR,
            wbb_bte => dBusWishbone_BTE,
            wbb_cti => dBusWishbone_CTI,
            wbo_cyc => Wishbone_CYC,
            wbo_stb => Wishbone_STB,
            wbo_ack => Wishbone_ACK,
            wbo_we => Wishbone_WE,
            wbo_addr => Wishbone_ADR,
            wbo_d_miso => Wishbone_DAT_MISO,
            wbo_d_mosi => Wishbone_DAT_MOSI,
            wbo_sel => Wishbone_SEL,
            wbo_err => Wishbone_ERR,
            wbo_bte => Wishbone_BTE,
            wbo_cti => Wishbone_CTI,
            clock => soft_cpu_clock);

        process (all)
        begin
            if Wishbone_ADR(29 downto 14) = (x"0003") then
                uart_sel <= '1';
            else
                uart_sel <= '0';
            end if;

            if Wishbone_ADR(29 downto 14) = (x"0001") then
                bios_sel <= '1';
            else
                bios_sel <= '0';
            end if;

            if sim then
                Wishbone_DAT_MISO <= (others => 'X');
            else
                Wishbone_DAT_MISO <= (others => '0');
            end if;

            Wishbone_ERR <= '1';
            Wishbone_ACK <= '0';

            if uart_sel then
                uart_wb_we <= Wishbone_WE;
                Wishbone_DAT_MISO <= uart_wb_d_miso;
                Wishbone_ACK <= uart_wb_ack;
                Wishbone_ERR <= uart_wb_err;
				uart_wb_cyc <= Wishbone_CYC;
				uart_wb_stb <= Wishbone_STB;
            else
                uart_wb_we <= '0';
				uart_wb_cyc <= '0';
				uart_wb_stb <= '0';
            end if;

            if bios_sel then
                Wishbone_DAT_MISO <= bios_data;
                Wishbone_ACK <= bios_wb_ack;
                Wishbone_ERR <= '0';
            end if;
        end process;

        serial: entity work.uart generic map(
            FREQ => FREQ) port map(
            wb_ack => uart_wb_ack,
            wb_d_miso => uart_wb_d_miso,
            wb_d_mosi => dBusWishbone_DAT_MOSI,
            wb_err => uart_wb_err,
            wb_addr => dBusWishbone_ADR(3 downto 0),
            wb_bte => dBusWishbone_BTE,
            wb_cti => dBusWishbone_CTI,
            wb_cyc => uart_wb_cyc,
            wb_sel => dBusWishbone_SEL,
            wb_stb => uart_wb_stb,
            wb_we => uart_wb_we,
            clock => soft_cpu_clock,
            tx => uart_tx,
            rx => uart_rx);
    end generate;

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
	
	process (memory_clock)
	begin
		if rising_edge(memory_clock) then
			if reset_sync = '1' then
				cpu_dready <= '0';
			elsif cpu_ram_cs or cpu_cartridge_cs then
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
		clockbuf => clockbuf,
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

	ppu3: entity work.nes_tripler generic map(
		sim => sim) port map(
		hdmi_line_ready => hdmi_line_ready,
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

