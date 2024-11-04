library IEEE;
use ieee.std_logic_1164.all;
use ieee.std_logic_misc.all;

use IEEE.NUMERIC_STD.ALL;

entity nes_ppu is
	Generic (
		ramtype: string := "sram");
   Port (clock : in STD_LOGIC;
         reset : in STD_LOGIC;
			cpu_addr: in std_logic_vector(2 downto 0);
			cpu_cs: in std_logic;
			cpu_rw: in std_logic;
			cpu_dout: in std_logic_vector(7 downto 0);
			cpu_din: out std_logic_vector(7 downto 0);
			cpu_mem_clock: in std_logic;
			int: out std_logic;
			ppu_ale: out std_logic;
			ppu_ad: out std_logic_vector(7 downto 0);
			ppu_a: out std_logic_vector(5 downto 0);
			ppu_din: in std_logic_vector(7 downto 0);
			ppu_rd: out std_logic;
			ppu_wr: out std_logic
			);
end nes_ppu;

architecture Behavioral of nes_ppu is
type REGISTERS is array (7 downto 0) of std_logic_vector (7 downto 0);
type OAM_DATA is array(255 downto 0) of std_logic_vector(7 downto 0);
type PALETTE_DATA is array(31 downto 0) of std_logic_vector(5 downto 0);
signal regs: REGISTERS;
signal oam: OAM_DATA;
signal palette: PALETTE_DATA;

signal oam_address: std_logic_vector(7 downto 0);

signal read_counter: std_logic_vector(19 downto 0);
constant READ_COUNTER_RESET: std_logic_vector := "10101010101010101010";

constant REG0_VRAM_ADDRESS_INCREMENT : integer := 4;

signal pending_vram_read: std_logic_vector(15 downto 0);
signal data_buffer: std_logic_vector(7 downto 0);

signal vram_address: std_logic_vector(15 downto 0);
signal palette_addr: std_logic_vector(4 downto 0);

signal address_bit: std_logic;
signal vblank_clear: std_logic;
begin

	process (cpu_mem_clock)
	begin
		if rising_edge(cpu_mem_clock) then
			read_counter <= std_logic_vector(unsigned(read_counter) - 1);
			if cpu_cs then
				if cpu_rw then
					case cpu_addr is
						when "010" =>
							cpu_din(7 downto 6) <= regs(2)(7 downto 6);
							address_bit <= '0';
							vblank_clear <= '1';
							read_counter <= READ_COUNTER_RESET;
						when "100" =>
							cpu_din <= oam(to_integer(unsigned(oam_address)));
							if oam_address(1 downto 0) = "10" then
								cpu_din(2) <= '0';
								cpu_din(3) <= '0';
								cpu_din(4) <= '0';
								read_counter <= READ_COUNTER_RESET;
							end if;
						when "111" =>
							pending_vram_read <= vram_address;
							if vram_address < x"3f00" then
								cpu_din <= data_buffer;
								read_counter <= READ_COUNTER_RESET;
							else
								pending_vram_read(15) <= '0';
								pending_vram_read(14) <= '0';
								pending_vram_read(12) <= '0';
								cpu_din(5 downto 0) <= palette(to_integer(unsigned(palette_addr)));
							end if;
							if regs(0)(REG0_VRAM_ADDRESS_INCREMENT) then
								vram_address <= "00" & std_logic_vector(unsigned(vram_address(13 downto 0)) + 1);
							else
								vram_address <= "00" & std_logic_vector(unsigned(vram_address(13 downto 0)) + 32);
							end if;
						when others =>
					end case;
				else
					
				end if;
			end if;
		end if;
	end process;

end Behavioral;