library IEEE;
use ieee.std_logic_1164.all;
use ieee.std_logic_misc.all;
use IEEE.NUMERIC_STD.ALL;

entity nes_ppu is
	Generic (
		random_noise: in std_logic := '1';
		ramtype: string := "sram");
   Port (r_out: out std_logic_vector(7 downto 0);
			g_out: out std_logic_vector(7 downto 0);
			b_out: out std_logic_vector(7 downto 0);
			pixel_valid: out std_logic;
			hstart: out std_logic;
			vstart: out std_logic;
			row: out std_logic_vector(8 downto 0);
			column: out std_logic_vector(8 downto 0);
			clock : in STD_LOGIC;
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

type PALETTE_LOOKUP_DATA is array (63 downto 0) of std_logic_vector(7 downto 0);

signal palette_r: PALETTE_LOOKUP_DATA := (
	std_logic_vector(to_unsigned(84, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(8, 8)),
	std_logic_vector(to_unsigned(48, 8)),
	std_logic_vector(to_unsigned(68, 8)),
	std_logic_vector(to_unsigned(92, 8)),
	std_logic_vector(to_unsigned(84, 8)),
	std_logic_vector(to_unsigned(60, 8)),
	std_logic_vector(to_unsigned(32, 8)),
	std_logic_vector(to_unsigned(8, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	
	std_logic_vector(to_unsigned(152, 8)),
	std_logic_vector(to_unsigned(8, 8)),
	std_logic_vector(to_unsigned(48, 8)),
	std_logic_vector(to_unsigned(92, 8)),
	std_logic_vector(to_unsigned(136, 8)),
	std_logic_vector(to_unsigned(160, 8)),
	std_logic_vector(to_unsigned(152, 8)),
	std_logic_vector(to_unsigned(120, 8)),
	std_logic_vector(to_unsigned(84, 8)),
	std_logic_vector(to_unsigned(40, 8)),
	std_logic_vector(to_unsigned(8, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	
	std_logic_vector(to_unsigned(236, 8)),
	std_logic_vector(to_unsigned(76, 8)),
	std_logic_vector(to_unsigned(120, 8)),
	std_logic_vector(to_unsigned(176, 8)),
	std_logic_vector(to_unsigned(228, 8)),
	std_logic_vector(to_unsigned(236, 8)),
	std_logic_vector(to_unsigned(236, 8)),
	std_logic_vector(to_unsigned(212, 8)),
	std_logic_vector(to_unsigned(160, 8)),
	std_logic_vector(to_unsigned(116, 8)),
	std_logic_vector(to_unsigned(76, 8)),
	std_logic_vector(to_unsigned(56, 8)),
	std_logic_vector(to_unsigned(56, 8)),
	std_logic_vector(to_unsigned(60, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	
	std_logic_vector(to_unsigned(236, 8)),
	std_logic_vector(to_unsigned(168, 8)),
	std_logic_vector(to_unsigned(188, 8)),
	std_logic_vector(to_unsigned(212, 8)),
	std_logic_vector(to_unsigned(236, 8)),
	std_logic_vector(to_unsigned(236, 8)),
	std_logic_vector(to_unsigned(236, 8)),
	std_logic_vector(to_unsigned(228, 8)),
	std_logic_vector(to_unsigned(204, 8)),
	std_logic_vector(to_unsigned(180, 8)),
	std_logic_vector(to_unsigned(168, 8)),
	std_logic_vector(to_unsigned(152, 8)),
	std_logic_vector(to_unsigned(160, 8)),
	std_logic_vector(to_unsigned(160, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8))
);

signal palette_g: PALETTE_LOOKUP_DATA := (
	std_logic_vector(to_unsigned(84, 8)),
	std_logic_vector(to_unsigned(30, 8)),
	std_logic_vector(to_unsigned(16, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(4, 8)),
	std_logic_vector(to_unsigned(24, 8)),
	std_logic_vector(to_unsigned(42, 8)),
	std_logic_vector(to_unsigned(58, 8)),
	std_logic_vector(to_unsigned(64, 8)),
	std_logic_vector(to_unsigned(60, 8)),
	std_logic_vector(to_unsigned(50, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	
	std_logic_vector(to_unsigned(150, 8)),
	std_logic_vector(to_unsigned(76, 8)),
	std_logic_vector(to_unsigned(50, 8)),
	std_logic_vector(to_unsigned(30, 8)),
	std_logic_vector(to_unsigned(20, 8)),
	std_logic_vector(to_unsigned(20, 8)),
	std_logic_vector(to_unsigned(34, 8)),
	std_logic_vector(to_unsigned(60, 8)),
	std_logic_vector(to_unsigned(90, 8)),
	std_logic_vector(to_unsigned(114, 8)),
	std_logic_vector(to_unsigned(124, 8)),
	std_logic_vector(to_unsigned(118, 8)),
	std_logic_vector(to_unsigned(102, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	
	std_logic_vector(to_unsigned(238, 8)),
	std_logic_vector(to_unsigned(154, 8)),
	std_logic_vector(to_unsigned(124, 8)),
	std_logic_vector(to_unsigned(98, 8)),
	std_logic_vector(to_unsigned(84, 8)),
	std_logic_vector(to_unsigned(88, 8)),
	std_logic_vector(to_unsigned(106, 8)),
	std_logic_vector(to_unsigned(136, 8)),
	std_logic_vector(to_unsigned(170, 8)),
	std_logic_vector(to_unsigned(196, 8)),
	std_logic_vector(to_unsigned(208, 8)),
	std_logic_vector(to_unsigned(204, 8)),
	std_logic_vector(to_unsigned(180, 8)),
	std_logic_vector(to_unsigned(60, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	
	std_logic_vector(to_unsigned(238, 8)),
	std_logic_vector(to_unsigned(204, 8)),
	std_logic_vector(to_unsigned(188, 8)),
	std_logic_vector(to_unsigned(178, 8)),
	std_logic_vector(to_unsigned(174, 8)),
	std_logic_vector(to_unsigned(174, 8)),
	std_logic_vector(to_unsigned(180, 8)),
	std_logic_vector(to_unsigned(196, 8)),
	std_logic_vector(to_unsigned(210, 8)),
	std_logic_vector(to_unsigned(222, 8)),
	std_logic_vector(to_unsigned(2226, 8)),
	std_logic_vector(to_unsigned(226, 8)),
	std_logic_vector(to_unsigned(214, 8)),
	std_logic_vector(to_unsigned(162, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8))
);

signal palette_b: PALETTE_LOOKUP_DATA := (
	std_logic_vector(to_unsigned(84, 8)),
	std_logic_vector(to_unsigned(1116, 8)),
	std_logic_vector(to_unsigned(1144, 8)),
	std_logic_vector(to_unsigned(136, 8)),
	std_logic_vector(to_unsigned(100, 8)),
	std_logic_vector(to_unsigned(48, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(60, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	
	std_logic_vector(to_unsigned(152, 8)),
	std_logic_vector(to_unsigned(196, 8)),
	std_logic_vector(to_unsigned(236, 8)),
	std_logic_vector(to_unsigned(228, 8)),
	std_logic_vector(to_unsigned(176, 8)),
	std_logic_vector(to_unsigned(100, 8)),
	std_logic_vector(to_unsigned(32, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(40, 8)),
	std_logic_vector(to_unsigned(120, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	
	std_logic_vector(to_unsigned(236, 8)),
	std_logic_vector(to_unsigned(236, 8)),
	std_logic_vector(to_unsigned(236, 8)),
	std_logic_vector(to_unsigned(236, 8)),
	std_logic_vector(to_unsigned(236, 8)),
	std_logic_vector(to_unsigned(180, 8)),
	std_logic_vector(to_unsigned(100, 8)),
	std_logic_vector(to_unsigned(32, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(32, 8)),
	std_logic_vector(to_unsigned(108, 8)),
	std_logic_vector(to_unsigned(204, 8)),
	std_logic_vector(to_unsigned(60, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	
	std_logic_vector(to_unsigned(236, 8)),
	std_logic_vector(to_unsigned(236, 8)),
	std_logic_vector(to_unsigned(236, 8)),
	std_logic_vector(to_unsigned(236, 8)),
	std_logic_vector(to_unsigned(236, 8)),
	std_logic_vector(to_unsigned(212, 8)),
	std_logic_vector(to_unsigned(176, 8)),
	std_logic_vector(to_unsigned(144, 8)),
	std_logic_vector(to_unsigned(120, 8)),
	std_logic_vector(to_unsigned(120, 8)),
	std_logic_vector(to_unsigned(144, 8)),
	std_logic_vector(to_unsigned(180, 8)),
	std_logic_vector(to_unsigned(228, 8)),
	std_logic_vector(to_unsigned(160, 8)),
	std_logic_vector(to_unsigned(0, 8)),
	std_logic_vector(to_unsigned(0, 8))
);

signal regs: REGISTERS := (others => (others => '0'));
signal oam: OAM_DATA  := (others => (others => '0'));
signal palette: PALETTE_DATA := (others => (others => '0'));

signal oam_address: std_logic_vector(7 downto 0);

signal read_counter1: std_logic_vector(19 downto 0) := (others => '0');
signal read_counter2: std_logic_vector(19 downto 0) := (others => '0');
constant READ_COUNTER_RESET: std_logic_vector := "10101010101010101010";

constant REG0_VRAM_ADDRESS_INCREMENT : integer := 2;
constant REG0_BACKGROUND_PATTERNTABLE_BASE : integer := 4;

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

signal write_ignore_counter: std_logic_vector(14 downto 0);

signal should_eval_sprites: std_logic;

signal scanline_number: std_logic_vector(8 downto 0) := std_logic_vector(to_unsigned(260, 9));
signal scanline_cycle: std_logic_vector(8 downto 0) := (others => '0');
signal frame_odd: std_logic := '0';

signal background_pixel : std_logic_vector(6 downto 0);
signal sprite_pixel: std_logic_vector(6 downto 0);
signal palette_pixel: std_logic_vector(5 downto 0);
signal pixel: std_logic_vector(5 downto 0);
signal priority_sprite: std_logic;

signal bg_tile_calc1: std_logic_vector(3 downto 0);
signal bg_tile_index: std_logic_vector(2 downto 0);

signal line_visible: std_logic;
signal line_post_visible: std_logic;
signal line_vblank: std_logic;
signal line_pre_visible: std_logic;

signal column_first: std_logic;
signal column_active: std_logic;
signal column_sprites: std_logic;
signal column_prefetch: std_logic;
signal column_dummy: std_logic;

signal cycle_active: std_logic_vector(8 downto 0);
signal cycle_sprite: std_logic_vector(8 downto 0);
signal cycle_prefetch: std_logic_vector(8 downto 0);
signal cycle_dummy: std_logic_vector(8 downto 0);

signal fetch_bg: std_logic;
signal fetch_sprite: std_logic;
signal fetch_dummy: std_logic;
signal fetch_idle: std_logic;

signal bg_fetch_cycle: std_logic_vector(8 downto 0);

signal prev_nametable_data: std_logic_vector(7 downto 0);
signal nametable_data: std_logic_vector(7 downto 0);
signal attributetable_data: std_logic_vector(7 downto 0);
signal patterntable_tile: std_logic_vector(15 downto 0);

signal vblank_clear_toggle: std_logic := '0';
signal vblank_clear_done: std_logic := '0';

signal random_data: std_logic_vector(31 downto 0);

begin
	process (all)
	begin
		palette_addr <= vram_address(4 downto 0);
		case vram_address(4 downto 0) is
			when "10000" | "10100" | "11000" | "11100" => palette_addr(4) <= '0';
			when others =>
		end case;
	end process;
	
	process (all)
	begin
		column <= cycle_active(8 downto 0);
		row <= scanline_number(8 downto 0);
		if line_visible and column_active then
			pixel_valid <= '1';
		else
			pixel_valid <= '0';
		end if;
		if (line_visible or line_post_visible or line_vblank) and column_first then
			hstart <= '1';
		else
			hstart <= '0';
		end if;
		if line_pre_visible and column_first then
			vstart <= '1';
		else
			vstart <= '0';
		end if;
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
		cycle_active <= std_logic_vector(unsigned(scanline_cycle) - 1);
		cycle_sprite <= std_logic_vector(unsigned(scanline_cycle) - 257);
		cycle_prefetch <= std_logic_vector(unsigned(scanline_cycle) - 321);
		cycle_dummy <= std_logic_vector(unsigned(scanline_cycle) - 337);
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
	
	process (all)
	begin
		if scanline_number < std_logic_vector(to_unsigned(240, 9)) then
			line_visible <= '1';
			line_post_visible <= '0';
			line_vblank <= '0';
			line_pre_visible <= '0';
		elsif scanline_number = std_logic_vector(to_unsigned(240, 9)) then
			line_visible <= '0';
			line_post_visible <= '1';
			line_vblank <= '0';
			line_pre_visible <= '0';
		elsif scanline_number < std_logic_vector(to_unsigned(261, 9)) then
			line_visible <= '0';
			line_post_visible <= '0';
			line_vblank <= '1';
			line_pre_visible <= '0';
		else
			line_visible <= '0';
			line_post_visible <= '0';
			line_vblank <= '0';
			line_pre_visible <= '1';
		end if;
	end process;
	
	process (all)
	begin
		if scanline_cycle = std_logic_vector(to_unsigned(0, 9)) then
			column_first <= '1';
			column_active <= '0';
			column_sprites <= '0';
			column_prefetch <= '0';
			column_dummy <= '0';
		elsif scanline_cycle < std_logic_vector(to_unsigned(257, 9)) then
			column_first <= '0';
			column_active <= '1';
			column_sprites <= '0';
			column_prefetch <= '0';
			column_dummy <= '0';
		elsif scanline_cycle < std_logic_vector(to_unsigned(321, 9)) then
			column_first <= '0';
			column_active <= '0';
			column_sprites <= '1';
			column_prefetch <= '0';
			column_dummy <= '0';
		elsif scanline_cycle < std_logic_vector(to_unsigned(337, 9)) then
			column_first <= '0';
			column_active <= '0';
			column_sprites <= '0';
			column_prefetch <= '1';
			column_dummy <= '0';
		else
			column_first <= '0';
			column_active <= '0';
			column_sprites <= '0';
			column_prefetch <= '0';
			column_dummy <= '1';
		end if;
	end process;
	
	process (all)
	begin
		if column_active then
			bg_fetch_cycle <= cycle_active;
		else
			bg_fetch_cycle <= cycle_prefetch;
		end if;
	end process;
	
	process (all)
	begin
		fetch_bg <= (line_visible or line_pre_visible) and (column_active or column_prefetch);
		fetch_sprite <= (line_visible or line_pre_visible) and column_sprites;
		fetch_dummy <= (line_visible or line_pre_visible) and column_dummy;
		fetch_idle <= (line_post_visible or line_vblank) and not column_first;
	end process;
	
	process (all)
	begin
		--TODO color emphasis not yet implemented
		r_out <= palette_r(to_integer(unsigned(pixel)));
		g_out <= palette_g(to_integer(unsigned(pixel)));
		b_out <= palette_b(to_integer(unsigned(pixel)));
		
		if scanline_cycle(0) then
			r_out <= "11111111";
		end if;
		if scanline_number(0) then
			g_out <= "11111111";
		end if;
        if scanline_cycle(0) or scanline_number(0) then
			b_out <= "00000000";
		end if;
		
		if vram_address(15 downto 8) = x"3f" then
			if not regs(1)(REG1_DRAW_BACKGROUND) and not regs(1)(REG1_DRAW_SPRITES) then
				palette_pixel <= palette(to_integer(unsigned(vram_address(5 downto 0))));
			else
				palette_pixel <= palette(0);
			end if;
		else
			palette_pixel <= palette(0);
		end if;
		
		if random_noise then
			pixel <= random_data(5 downto 0);
		elsif priority_sprite then
			if sprite_pixel(6) then
				pixel <= sprite_pixel(5 downto 0);
			elsif background_pixel(6) then
				pixel <= background_pixel(5 downto 0);
			else
				pixel <= palette_pixel;
			end if;
		else
			if background_pixel(6) then
				pixel <= background_pixel(5 downto 0);
			elsif sprite_pixel(6) then
				pixel <= sprite_pixel(5 downto 0);
			else
				pixel <= palette_pixel;
			end if;
		end if;
	end process;
	
	process (all)
	begin
		bg_tile_calc1 <= std_logic_vector(unsigned('0' & scrollx(2 downto 0)) + unsigned('0' & cycle_active(2 downto 0)));
		bg_tile_index <= std_logic_vector("111" - unsigned(cycle_active(2 downto 0)) - unsigned(scrollx(2 downto 0)));
	end process;
	
	process (reset, clock)
	begin
		if reset then
		elsif rising_edge(clock) then
			if line_visible then
			end if;
		end if;
	end process;
	
	process (reset, clock)
	begin
		if reset then
		elsif rising_edge(clock) then
			if fetch_bg then
				ppu_ale <= not bg_fetch_cycle(0);
				case bg_fetch_cycle(2 downto 1) is
					when "00" =>
						if not bg_fetch_cycle(0) then
							ppu_ad <= vram_address(7 downto 0);
							ppu_a(5 downto 4) <= "10";
							ppu_a(3 downto 0) <= vram_address(11 downto 8);
							ppu_rd <= '1';
							ppu_wr <= '0';
						else
							prev_nametable_data <= nametable_data;
							nametable_data <= ppu_din;
						end if;
					when "01" =>
						if not bg_fetch_cycle(0) then
							ppu_a(5 downto 4) <= "10";
							ppu_a(3 downto 2) <= vram_address(11 downto 10);
							ppu_a(1 downto 0) <= "11";
							ppu_ad(7 downto 6) <= "00";
							ppu_ad(5 downto 3) <= vram_address(6 downto 4);
							ppu_ad(2 downto 0) <= vram_address(4 downto 2);
							ppu_rd <= '1';
							ppu_wr <= '0';
						else
							attributetable_data <= ppu_din;
						end if;
					when "10" =>
						if not bg_fetch_cycle(0) then
							ppu_a(5) <= '0';
							ppu_a(4) <= regs(0)(REG0_BACKGROUND_PATTERNTABLE_BASE);
						else
							patterntable_tile(7 downto 0) <= ppu_din;
						end if;
					when others =>
						if not bg_fetch_cycle(0) then
							ppu_a(5) <= '0';
							ppu_a(4) <= regs(0)(REG0_BACKGROUND_PATTERNTABLE_BASE);
						else
							patterntable_tile(15 downto 8) <= ppu_din;
						end if;
				end case;
			end if;
		end if;
	end process;
	
	process (clock)
	begin
		if rising_edge(clock) then
			if scanline_cycle = std_logic_vector(to_unsigned(1, 9)) then
				if scanline_number = std_logic_vector(to_unsigned(241, 9)) then
					regs(2)(7) <= '1';
				elsif line_pre_visible then
					regs(2)(7) <= '0';
				elsif vblank_clear_toggle xor vblank_clear_done then
					regs(2)(7) <= '0';
					vblank_clear_done <= not vblank_clear_done;
				end if;
			end if;
		end if;
	end process;
	
	process (reset, clock)
	begin
		if reset then
			--TODO?
		elsif rising_edge(clock) then
			if regs(1)(REG1_DRAW_BACKGROUND) or regs(1)(REG1_DRAW_SPRITES) then
				if line_visible or line_pre_visible then
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
							if vram_address(15 downto 13) = "111" then
								vram_address(15 downto 13) <= "000";
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
				if line_pre_visible then
					if scanline_cycle >= std_logic_vector(to_unsigned(280, 9))
						and scanline_cycle <= std_logic_vector(to_unsigned(304, 9)) then
						--transfer vertical position
						vram_address(14 downto 11) <= temporary_vram_address(14 downto 11);
						vram_address(9 downto 5) <= temporary_vram_address(9 downto 5);
					end if;
				end if;
			end if;
			scanline_cycle <= std_logic_vector(unsigned(scanline_cycle) + 1);
			if scanline_cycle = std_logic_vector(to_unsigned(340, 9)) then
				scanline_cycle <= (others => '0');
				scanline_number <= std_logic_vector(unsigned(scanline_number) + 1);
				if line_pre_visible then
					frame_odd <= not frame_odd;
					scanline_number <= (others => '0');
					if frame_odd and regs(1)(REG1_DRAW_BACKGROUND) then
						scanline_cycle <= std_logic_vector(to_unsigned(1, 9));
					end if;
				end if;
			end if;
		end if;
	end process;

	random: entity work.lfsr32 port map(
		clock => clock,
		dout => random_data);

	process (cpu_mem_clock, reset)
	begin
		if reset then
			regs(0) <= x"00";
			regs(1) <= x"00";
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
				if cpu_rw then --read
					case cpu_addr is
						when "010" =>
							cpu_din(7 downto 6) <= regs(2)(7 downto 6);
							address_bit <= '0';
							vblank_clear_toggle <= not vblank_clear_toggle;
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
				else	--write
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
								--vram_address <= "00" & std_logic_vector(unsigned(vram_address(13 downto 0)) + 1);
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