library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity nes_cartridge is
	Generic (
		ramtype: string := "sram";
		unified_ram: std_logic := '0');
   Port (ppu_data_in: in std_logic_vector(7 downto 0);
			ppu_data_out: out std_logic_vector(7 downto 0);
			ppu_addr: in std_logic_vector(13 downto 0);
			ppu_addr_a13_n: in std_logic;
			ppu_wr: in std_logic;
			ppu_rd: in std_logic;
			ciram_a10: out std_logic;
			ciram_ce: out std_logic;
			irq: out std_logic;
			cpu_rw: in std_logic;
			romsel: in std_logic;
			cpu_data_in: out std_logic_vector(7 downto 0);
			cpu_data_out: in std_logic_vector(7 downto 0);
			cpu_addr: in std_logic_vector(15 downto 0);
			m2: in std_logic;
			clock: in std_logic;
			write_signal: in std_logic := '0';
			write_address: in std_logic_vector(19 downto 0) := (others=>'0');
			write_value: in std_logic_vector(7 downto 0) := (others=>'0');
			write_trigger: in std_logic := '0';
			write_cs: in std_logic_vector(1 downto 0) := (others=>'0'));
end nes_cartridge;

architecture Behavioral of nes_cartridge is
	signal mapper: std_logic_vector(15 downto 0) := x"0000";
	
	signal mirroring: std_logic;
	
	signal prg_rom_address: std_logic_vector(14 downto 0);
	signal prg_rom_data: std_logic_vector(7 downto 0);
	signal prg_rom_cs: std_logic;
	
	signal chr_rom_address: std_logic_vector(12 downto 0);
	signal chr_rom_data: std_logic_vector(7 downto 0);
	signal chr_rom_cs: std_logic;
begin
	process (all)
	begin
		chr_rom_cs <= '0';
		case mapper is
			when x"0000" =>
				if write_signal then
					prg_rom_address(14 downto 0) <= write_address(14 downto 0);
				else
					prg_rom_address(14 downto 0) <= cpu_addr(14 downto 0);
				end if;
				cpu_data_in <= prg_rom_data;
				if cpu_addr(15) = '1' then
					prg_rom_cs <= '1';
				else
					prg_rom_cs <= '0';
				end if;
			when others =>
				prg_rom_cs <= '0';
		end case;
	end process;
	
	memory: if (unified_ram = '0' and ramtype = "sram") generate
		prg_rom: entity work.clocked_sram 
			generic map (bits => 14)
			port map(
				clock => m2,
				address => prg_rom_address(13 downto 0),
				rw => '1',
				cs => prg_rom_cs,
				dout => prg_rom_data,
				din => (others=>'0'));
		chr_rom: entity work.clocked_sram
			generic map (bits => 13)
			port map(
				clock => m2,
				address => (others => '0'),
				rw => not ppu_wr,
				cs => chr_rom_cs,
				dout => chr_rom_data,
				din => ppu_data_in);
	end generate;

end Behavioral;

