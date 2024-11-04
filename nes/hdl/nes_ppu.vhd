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

signal read_counter1: std_logic_vector(19 downto 0);
signal read_counter2: std_logic_vector(19 downto 0);
constant READ_COUNTER_RESET: std_logic_vector := "10101010101010101010";

constant REG0_VRAM_ADDRESS_INCREMENT : integer := 4;

constant REG1_DRAW_SPRITES : integer := 4;
constant REG1_DRAW_SPRITES_FIRST_COLUMN : integer := 2;
constant REG1_DRAW_BACKGROUND : integer := 3;
constant REG1_DRAW_BACKGROUND_FIRST_COLUMN : integer := 1;

signal pending_vram_read: std_logic_vector(15 downto 0);
signal pending_vram_write: std_logic_vector(7 downto 0);
signal data_buffer: std_logic_vector(7 downto 0);

signal vram_address: std_logic_vector(15 downto 0) := (others => '0');
signal temporary_vram_address: std_logic_vector(15 downto 0) := (others => '0');
signal scrollx: std_logic_vector(7 downto 0) := (others => '0');

signal palette_addr: std_logic_vector(4 downto 0) := (others => '0');

signal address_bit: std_logic;
signal vblank_clear: std_logic;

signal write_ignore_counter: std_logic_vector(14 downto 0);

signal should_eval_sprites: std_logic;

signal scanline_number: std_logic_vector(8 downto 0) := (others => '0');
signal scanline_cycle: std_logic_vector(8 downto 0) := (others => '0');
signal frame_odd: std_logic := '0';
begin
	process (all)
	begin
		palette_addr <= vram_address(4 downto 0);
		case vram_address(4 downto 0) is
			when "10000" | "10100" | "11000" | "11100" => palette_addr(4) <= '0';
			when others =>
		end case;
	end process;
	
	process (reset, clock)
	begin
		if reset then
			write_ignore_counter <= std_logic_vector(to_unsigned(29658, 15));
		elsif rising_edge(clock) then
			if write_ignore_counter /= "000000000000000" then
				write_ignore_counter <= std_logic_vector(unsigned(write_ignore_counter) - 1);
			end if;
		end if;
	end process;
	
	process (all)
	begin
		should_eval_sprites <= regs(1)(REG1_DRAW_SPRITES) or regs(1)(REG1_DRAW_SPRITES_FIRST_COLUMN) or regs(1)(REG1_DRAW_BACKGROUND) or regs(1)(REG1_DRAW_BACKGROUND_FIRST_COLUMN);
	end process;
	
	process (reset, clock)
	begin
		if reset then
			--TODO?
		elsif rising_edge(clock) then
			if should_eval_sprites then
				--TODO
			end if;
		end if;
	end process;
	
	process (reset, clock)
	begin
		if reset then
			--TODO?
		elsif rising_edge(clock) then
			if regs(1)(REG1_DRAW_BACKGROUND) or regs(1)(REG1_DRAW_SPRITES) then
				if scanline_number < std_logic_vector(to_unsigned(240, 9)) or scanline_number = std_logic_vector(to_unsigned(261, 9)) then
					if (scanline_cycle >= std_logic_vector(to_unsigned(328, 9)) or scanline_cycle < std_logic_vector(to_unsigned(256, 9)))
						and scanline_cycle(8 downto 3) /= "000000"
						and scanline_cycle(2 downto 0) = "000" then
						--increment horizontal position
						if vram_address(4 downto 0) = "11111" then
							vram_address(10) <= not vram_address(10);
							vram_address(4 downto 0) <= "00000";
						else
							vram_address(4 downto 0) <= std_logic_vector(unsigned(vram_address(4 downto 0)) + 1);
						end if;
					end if;
					case scanline_cycle is
						when std_logic_vector(to_unsigned(256, 9)) =>
							--increment vertical position
							if vram_address(15 downto 12) = "111" then
								vram_address(15 downto 12) <= "000";
								if vram_address(9 downto 5) = std_logic_vector(to_unsigned(29, 5)) then
									vram_address(11) <= not vram_address(11);
								end if;
								case vram_address(9 downto 5) is
									when std_logic_vector(to_unsigned(29, 5)) | std_logic_vector(to_unsigned(31, 5)) =>
										vram_address(9 downto 5) <= "00000";
									when others =>
										vram_address(9 downto 5) <= std_logic_vector(unsigned(vram_address(9 downto 5)) + 1);
								end case;
							else
								vram_address(15 downto 12) <= std_logic_vector(unsigned(vram_address(15 downto 12)) + 1);
							end if;
						when std_logic_vector(to_unsigned(257, 9)) =>
							--transfer horizontal position
							vram_address(10) <= temporary_vram_address(10);
							vram_address(4 downto 0) <= temporary_vram_address(4 downto 0);
						when others =>
					end case;
				end if;
				if scanline_number = std_logic_vector(to_unsigned(261, 9)) 
					and scanline_cycle >= std_logic_vector(to_unsigned(280, 9))
					and scanline_cycle <= std_logic_vector(to_unsigned(304, 9)) then
					--transfer vertical position
					vram_address(14 downto 11) <= temporary_vram_address(14 downto 11);
					vram_address(9 downto 5) <= temporary_vram_address(9 downto 5);
				end if;
			end if;
			scanline_cycle <= std_logic_vector(unsigned(scanline_cycle) + 1);
			if scanline_cycle = std_logic_vector(to_unsigned(340, 9)) then
				scanline_cycle <= (others => '0');
				scanline_number <= std_logic_vector(unsigned(scanline_number) + 1);
				if scanline_number = std_logic_vector(to_unsigned(261, 9)) then
					frame_odd <= not frame_odd;
					scanline_number <= (others => '0');
					if frame_odd and regs(1)(REG1_DRAW_BACKGROUND) then
						scanline_cycle <= std_logic_vector(to_unsigned(1, 9));
					end if;
				end if;
			end if;
		end if;
	end process;

	process (cpu_mem_clock, reset)
	begin
		if reset then
			regs(0) <= x"ff";
			regs(1) <= x"ff";
		elsif rising_edge(cpu_mem_clock) then
			if read_counter1 /= x"00000" then
				read_counter1 <= std_logic_vector(unsigned(read_counter1) - 1);
			else
				cpu_din(7 downto 5) <= "000";
			end if;
			if read_counter2 /= x"00000" then
				read_counter2 <= std_logic_vector(unsigned(read_counter2) - 1);
			else
				cpu_din(4 downto 0) <= "00000";
			end if;
			if cpu_cs then
				if cpu_rw then
					case cpu_addr is
						when "010" =>
							cpu_din(7 downto 6) <= regs(2)(7 downto 6);
							address_bit <= '0';
							vblank_clear <= '1';
							read_counter2 <= READ_COUNTER_RESET;
						when "100" =>
							cpu_din <= oam(to_integer(unsigned(oam_address)));
							if oam_address(1 downto 0) = "10" then
								cpu_din(2) <= '0';
								cpu_din(3) <= '0';
								cpu_din(4) <= '0';
								read_counter1 <= READ_COUNTER_RESET;
								read_counter2 <= READ_COUNTER_RESET;
							end if;
						when "111" =>
							pending_vram_read <= vram_address;
							if vram_address < x"3f00" then
								cpu_din <= data_buffer;
								read_counter1 <= READ_COUNTER_RESET;
								read_counter2 <= READ_COUNTER_RESET;
							else
								pending_vram_read(15) <= '0';
								pending_vram_read(14) <= '0';
								pending_vram_read(12) <= '0';
								cpu_din(5 downto 0) <= palette(to_integer(unsigned(palette_addr)));
							end if;
							if regs(0)(REG0_VRAM_ADDRESS_INCREMENT) then
								--vram_address <= "00" & std_logic_vector(unsigned(vram_address(13 downto 0)) + 1);
							else
								--vram_address <= "00" & std_logic_vector(unsigned(vram_address(13 downto 0)) + 32);
							end if;
						when others =>
					end case;
				else
					cpu_din <= cpu_dout;
					read_counter1 <= READ_COUNTER_RESET;
					read_counter2 <= READ_COUNTER_RESET;
					case cpu_addr is
						when "011" =>
							oam_address <= cpu_dout;
						when "100" =>
							oam(to_integer(unsigned(oam_address))) <= cpu_dout;
							oam_address <= std_logic_vector(unsigned(oam_address) + 1);
						when "111" =>
							if vram_address < x"3f00" then
								pending_vram_write <= cpu_dout;
							else
								palette(to_integer(unsigned(palette_addr))) <= cpu_dout(5 downto 0);
								--TODO increment vram
							end if;
						when others =>
							--TODO check write ignore counter
							case cpu_addr is
								when "001" =>
									regs(1) <= cpu_dout;
								when "000" =>
									regs(0) <= cpu_dout;
									temporary_vram_address(11 downto 10) <= cpu_dout(1 downto 0);
									temporary_vram_address(15) <= '0';
								when "101" =>
									if not address_bit then
										temporary_vram_address(4 downto 0) <= cpu_dout(7 downto 3);
										scrollx <= "00000" & cpu_dout(2 downto 0);
									else
										temporary_vram_address(14 downto 12) <= cpu_dout(2 downto 0);
										temporary_vram_address(6 downto 2) <= cpu_dout(7 downto 3);
										temporary_vram_address(15) <= '0';
									end if;
									address_bit <= not address_bit;
								when "110" =>
									if not address_bit then
										regs(6) <= cpu_dout;
										temporary_vram_address(13 downto 8) <= cpu_dout(5 downto 0);
										temporary_vram_address(15 downto 14) <= "00";
									else
										temporary_vram_address(15 downto 0) <= x"00" & cpu_dout;
										--vram_address <= x"00" & cpu_dout;
									end if;
									address_bit <= not address_bit;
								when others =>
							end case;
					end case;
				end if;
			end if;
		end if;
	end process;

end Behavioral;