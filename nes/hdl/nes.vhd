library IEEE;
use IEEE.STD_LOGIC_1164.ALL;

entity nes is
    Port (
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
	signal cpu_dready: std_logic;
	signal cpu_rw: std_logic;
	signal memory_clock: std_logic;
	
	signal cpu_sram_din: std_logic_vector(7 downto 0);
	signal cpu_sram_dout: std_logic_vector(7 downto 0);
	signal cpu_ram_cs: std_logic;
	
	signal cpu_ppu_cs: std_logic;
	signal cpu_apu_cs: std_logic;
	
	signal cpu_cartridge_cs: std_logic;
	signal cpu_cartridge_din: std_logic_vector(7 downto 0);
	signal cpu_cartridge_dout: std_logic_vector(7 downto 0);
begin
	whocares <= clock;
	otherstuff <= cpu_address;
	cpu_memory_address <= cpu_address;
	cs_out <= cpu_ram_cs & cpu_ppu_cs & cpu_apu_cs & cpu_cartridge_cs;
	
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
	end process;
	
	process (memory_clock)
	begin
		if reset = '1' then
			cpu_dready <= '0';
		elsif rising_edge(memory_clock) then
			if cpu_ram_cs = '1' then
				cpu_dready <= '1';
			elsif cpu_cartridge_cs = '1' then
			   cpu_dready <= '1';
			else
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
	
	cpu: entity work.nes_cpu port map (
		clock => clock,
		memory_clock => memory_clock,
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
	
	cartridge: entity work.nes_cartridge port map (
		cpu_data_out => cpu_cartridge_dout,
		cpu_data_in => cpu_cartridge_din,
		cpu_addr => cpu_address,
		ppu_data_in => "00000000",
		ppu_addr => "00000000000000",
		ppu_addr_a13_n => '1',
		ppu_wr => '0',
		ppu_rd => '0',
		cpu_rw => cpu_rw,
		romsel => cpu_address(15),
		m2 => memory_clock,
		clock => clock
	);

end Behavioral;

