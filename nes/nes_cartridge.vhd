library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity nes_cartridge is
    Port (cic_out: out std_logic;
			cic_in: in std_logic;
			cic_clk: in std_logic;
			cic_rst: in std_logic;
			ppu_data: inout std_logic_vector(7 downto 0);
			ppu_addr: in std_logic_vector(13 downto 0);
			ppu_addr_13: in std_logic;
			ppu_wr: in std_logic;
			ppu_rd: in std_logic;
			ciram_a10: out std_logic;
			ciram_ce: out std_logic;
			exp: inout std_logic_vector(9 downto 0);
			irq: out std_logic;
			cpu_rw: in std_logic;
			romsel: in std_logic;
			cpu_data: inout std_logic_vector(7 downto 0);
			cpu_addr: in std_logic_vector(14 downto 0);
			m2: in std_logic;
			clock: in std_logic);
end nes_cartridge;

architecture Behavioral of nes_cartridge is
	signal recovered_address: std_logic_vector(15 downto 0);
begin
	recovered_address(14 downto 0) <= cpu_addr;
	recovered_address(15) <= not romsel;
	prg_rom: entity work.sram_init 
		generic map (num_bits => 15, filename => "rom.txt")
		port map(
			addr => cpu_addr(14 downto 0),
			oe => not cpu_rw,
			we => cpu_rw,
			cs => romsel,
			data => cpu_data);
	chr_rom: entity work.sram_init
		generic map (num_bits => 13, filename => "chr_rom.txt")
		port map(
			addr => (others => '0'),
			oe => '1',
			we => '1',
			cs => '1',
			data => ppu_data);
	ctg_ram: entity work.sram
		generic map (num_bits => 12)
		port map(
			addr => (others => '0'),
			oe => '1',
			we => '1',
			cs => '1',
			data => cpu_data);

end Behavioral;

