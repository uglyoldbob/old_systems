library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

-- This entity takes in two vsync signals where sync1 is always faster than sync2
-- It issues a pause signal, temporarily pausing the processing of the first stream until the second stream is caught up
entity frame_sync is
	Port (
		clock: in std_logic;
		sync1: in std_logic;
		sync2: in std_logic;
		hsync1: in std_logic;
		hsync2: in std_logic;
		pause: out std_logic);
end frame_sync;

architecture Behavioral of frame_sync is
	signal mode: std_logic := '1';
	signal mode2: std_logic;
	signal hsync2_count: integer range 0 to 3;
	signal frame_bit1: std_logic := '0';
	signal frame_bit2: std_logic := '0';
begin
	pause <= mode or mode2;
	process (clock)
	begin
		if rising_edge(clock) then
			if hsync2_count = 3 then
				mode2 <= '1';
			else
				mode2 <= '0';
			end if;
			if ((frame_bit1 = '1') xor (frame_bit2 = '1')) then
				mode <= '1';
			else
				mode <= '0';
			end if;
			if sync1 then
				frame_bit1 <= not frame_bit1;
			end if;
			if sync2 then
				frame_bit2 <= not frame_bit2;
			end if;
			if hsync1 ='1' then
				hsync2_count <= 3;
			elsif hsync2 ='1' and hsync2_count > 0 then
				hsync2_count <= hsync2_count - 1;
			end if;
		end if;
	end process;
end Behavioral;