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
			cpu_data_in_ready: out std_logic;
			cpu_addr: in std_logic_vector(15 downto 0);
			m2: in std_logic;
			clock: in std_logic;
			write_signal: in std_logic := '0';
			write_address: in std_logic_vector(19 downto 0) := (others=>'0');
			write_value: in std_logic_vector(7 downto 0) := (others=>'0');
			write_trigger: in std_logic := '0';
			write_rw: in std_logic;
			write_cs: in std_logic_vector(1 downto 0) := (others=>'0'));
end nes_cartridge;

architecture Behavioral of nes_cartridge is
	signal mapper: std_logic_vector(15 downto 0) := x"0000";
	
	signal mirroring: std_logic;

	signal prg_rom_din: std_logic_vector(7 downto 0);
	signal prg_rom_address: std_logic_vector(14 downto 0);
	signal prg_rom_data: std_logic_vector(7 downto 0);
	signal prg_rom_data_ready: std_logic;
	signal prg_rom_cs: std_logic;
	signal prg_rom_rw: std_logic;
	
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
				if write_signal then
					case write_cs is
						when "00" => 
							prg_rom_cs <= '1';
						when others => 
							prg_rom_cs <= '0';
					end case;
					prg_rom_rw <= write_rw;
					prg_rom_din <= write_value;
					cpu_data_in_ready <= '1';
				else
					prg_rom_din <= (others => '0');
					prg_rom_rw <= '1';
					if cpu_addr(15) = '1' then
						prg_rom_cs <= '1';
						cpu_data_in_ready <= prg_rom_data_ready;
					else
						prg_rom_cs <= '0';
						cpu_data_in_ready <= '1';
					end if;
				end if;
			when others =>
				prg_rom_address <= (others => '0');
				prg_rom_cs <= '0';
				cpu_data_in => (others => '0');
		end case;
	end process;
	
	memory: if (unified_ram = '0' and ramtype = "sram") generate
		prg_rom: entity work.clocked_sram 
			generic map (bits => 14)
			port map(
				clock => m2,
				fast_clock => clock,
				address => prg_rom_address(13 downto 0),
				rw => prg_rom_rw,
				cs => prg_rom_cs,
				dout => prg_rom_data,
				dout_valid => prg_rom_data_ready,
				din => prg_rom_din);
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

