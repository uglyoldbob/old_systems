library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity clocked_sram is
	Generic (
		bits: integer := 11;
		dbits: integer := 8;
		delay: integer:= 0);
	Port (
		clock: in std_logic;
		fast_clock: in std_logic := '0';
		cs: in std_logic;
		address: in std_logic_vector(bits-1 downto 0);
		rw: in std_logic;
		din: in std_logic_vector(dbits-1 downto 0);
		dout_valid: out std_logic;
		dout: out std_logic_vector(dbits-1 downto 0)
		);
end clocked_sram;

architecture Behavioral of clocked_sram is
type DELAY_ARRAY is array(delay-1 downto 0) of std_logic_vector (dbits-1 downto 0);
type RAM_ARRAY is array (2**bits-1 downto 0) of std_logic_vector (dbits-1 downto 0);
signal prev_address: std_logic_vector(bits-1 downto 0);
signal ready_signal: std_logic_vector(delay-1 downto 0);
signal ready_delay: std_logic;
signal dout_buffer: std_logic_vector(dbits-1 downto 0);
signal dout1: std_logic_vector(dbits-1 downto 0);
signal dout2: std_logic_vector(dbits-1 downto 0);
signal ram: RAM_ARRAY;
signal delay_data: DELAY_ARRAY;
begin
	process (all)
	begin
		if delay /= 0 then
			dout <= dout2;
			dout_valid <= ready_delay;
		else
			dout <= dout1;
			dout_valid <= '1';
		end if;
	end process;

	process (clock)
	begin
		if rising_edge(clock) then
			if cs then
				if not rw then
					ram(to_integer(unsigned(address))) <= din;
				else
					prev_address <= address;
					dout1 <= ram(to_integer(unsigned(address)));
					if delay /= 0 then
						dout_buffer <= ram(to_integer(unsigned(address)));
					end if;
				end if;
			end if;
		end if;
	end process;
	
	process (fast_clock)
	begin
		if rising_edge(fast_clock) then
			if cs then
				if prev_address = address then
					if delay = 1 then
						ready_signal(0) <= '1';
						ready_delay <= ready_signal(0);
					elsif delay /= 0 then
						ready_signal(delay-2) <= '1';
						for i in 0 to delay-3 loop
							ready_signal(i) <= ready_signal(i+1);
						end loop;
						ready_delay <= ready_signal(0);
					else
						ready_delay <= '1';
					end if;
				else
					for i in 0 to delay-1 loop
						ready_signal(i) <= '0';
					end loop;
					ready_delay <= '0';
				end if;
			end if;
			if delay = 1 then
				dout2 <= dout_buffer;
			elsif delay /= 0 then
				delay_data(delay-2) <= dout_buffer;
				for i in 0 to delay-3 loop
					delay_data(i) <= delay_data(i+1);
				end loop;
				dout2 <= delay_data(0);
			end if;
		end if;
	end process;
end Behavioral;
