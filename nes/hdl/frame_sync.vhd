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
		pause: out std_logic);
end frame_sync;

architecture Behavioral of frame_sync is
	signal mode: std_logic := '1';
begin
	pause <= mode;
	process (clock)
	begin
		if rising_edge(clock) then
			if not mode then
				if sync1 then
					mode <= '1';
				end if;
			else
				if sync2 then
					mode <= '0';
				end if;
			end if;
		end if;
	end process;
end Behavioral;