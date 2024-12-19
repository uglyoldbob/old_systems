library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

-- This entity takes in two vsync signals where sync1 is always faster than sync2
-- It issues a pause signal, temporarily pausing the processing of the first stream until the second stream is caught up
entity frame_sync is
	Port (
		clock: in std_logic;
		vsync1: in std_logic;
		vsync2: in std_logic;
		hsync1: in std_logic;
		hsync2: in std_logic;
		pause: out std_logic);
end frame_sync;

architecture Behavioral of frame_sync is
	constant MODE_WAIT_FOR_HDMI_LINE: integer := 0;
	constant MODE_WAIT_FOR_PPU_LINE: integer := 1;

	signal row_count: integer range 0 to 241 := 0;
	signal hdmi_row: integer range 0 to 720 := 0;

	signal mode: integer range 0 to 3 := MODE_WAIT_FOR_PPU_LINE;
	signal mode2: std_logic;
	signal hsync2_count: integer range 0 to 3;
begin
	pause <= mode2;
	process (clock)
	begin
		if rising_edge(clock) then
			case mode is
				when MODE_WAIT_FOR_HDMI_LINE =>
					mode2 <= '1';
				when others =>
					mode2 <= '0';
			end case;
			if vsync1 then
				row_count <= 0;
			elsif hsync1 then
				row_count <= row_count + 1;
			end if;
			if vsync2 then
				hdmi_row <= 0;
			elsif hsync2 then
				hdmi_row <= hdmi_row + 1;
			end if;
			if row_count = 241 then
				mode <= MODE_WAIT_FOR_PPU_LINE;
				row_count <= 0;
			end if;
			case mode is
				when MODE_WAIT_FOR_PPU_LINE =>
					if hsync1 then
						mode <= MODE_WAIT_FOR_HDMI_LINE;
					end if;
				when MODE_WAIT_FOR_HDMI_LINE =>
					if vsync2 = '1' or hsync2_count = 3 then
						hsync2_count <= 0;
						mode <= MODE_WAIT_FOR_PPU_LINE;
					end if;
				when others => null;
			end case;
			if hsync2 ='1' and hsync2_count < 3 then
				hsync2_count <= hsync2_count + 1;
			end if;
		end if;
	end process;
end Behavioral;