library IEEE;
use IEEE.STD_LOGIC_1164.ALL;

entity nes is
	Generic (
		ramtype: string := "sram";
		unified_ram: std_logic := '0');
   Port (
		write_signal: in std_logic := '0';
		write_address: in std_logic_vector(19 downto 0) := (others=>'0');
		write_value: in std_logic_vector(7 downto 0) := (others=>'0');
		write_trigger: in std_logic := '0';
		write_rw: in std_logic;
		write_cs: in std_logic_vector(1 downto 0) := (others=>'0');
		
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
	signal memory_clock: std_logic;
	signal memory_start: std_logic;
	
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
	
	signal cpu_apu_cs: std_logic;
	
	signal cpu_cartridge_cs: std_logic;
	signal cpu_cartridge_din: std_logic_vector(7 downto 0);
	signal cpu_cartridge_din_ready: std_logic;
	signal cpu_cartridge_dout: std_logic_vector(7 downto 0);
	
	signal pause: std_logic;
begin
	whocares <= clock;
	otherstuff <= cpu_address;
	cpu_memory_address <= cpu_address;
	cs_out <= cpu_ram_cs & cpu_ppu_cs & cpu_apu_cs & cpu_cartridge_cs;
	
	d_memory_clock <= memory_clock;
	pause <= write_signal or (cpu_memory_clock and not cpu_din_ready);
	
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
		if cpu_ram_cs = '1' then
			cpu_din <= cpu_sram_dout;
		elsif cpu_cartridge_cs = '1' then
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
			if write_signal then
				memory_clock <= write_trigger;
			else
				memory_clock <= cpu_memory_clock;
			end if;
		end if;
	end process;
	
	process (reset, memory_clock)
	begin
		if reset = '1' then
			cpu_dready <= '0';
		elsif rising_edge(memory_clock) then
			if cpu_ram_cs or cpu_cartridge_cs then
				cpu_dready <= '1';
			end if;
		end if;
	end process;
	
	ram_nonunified: if (unified_ram = '0' and ramtype = "sram") generate
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
	end generate;
	
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
		memory_start => memory_start,
		memory_cycle_done => cpu_dready,
		rw => cpu_rw,
		oe => cpu_oe,
		reset => reset,
		din => cpu_din,
		dout => cpu_dout,
		nmi => '1',
		irq => '1',
		tst => '0',
		address => cpu_address);
	
	ppu: entity work.nes_ppu generic map(
		ramtype => ramtype) port map (
		clock => ppu_clock,
		reset => reset,
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
	
	cartridge: entity work.nes_cartridge generic map(
		ramtype => ramtype,
		unified_ram => unified_ram) port map (
		cpu_data_out => cpu_cartridge_dout,
		cpu_data_in => cpu_cartridge_din,
		cpu_data_in_ready => cpu_cartridge_din_ready,
		cpu_addr => cpu_address,
		ppu_data_in => "00000000",
		ppu_addr => "00000000000000",
		ppu_addr_a13_n => '1',
		ppu_wr => '0',
		ppu_rd => '0',
		cpu_rw => cpu_rw,
		romsel => cpu_address(15),
		m2 => memory_clock,
		clock => clock,
		write_signal => write_signal,
		write_address => write_address,
		write_value => write_value,
		write_trigger => write_trigger,
		write_rw => write_rw,
		write_cs => write_cs
	);

end Behavioral;

