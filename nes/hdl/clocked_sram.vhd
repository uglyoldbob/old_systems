library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity clocked_sram is
	Generic (
		bits: integer := 11;
		dbits: integer := 8);
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
type RAM_ARRAY is array (2**bits-1 downto 0) of std_logic_vector (dbits-1 downto 0);
signal ram: RAM_ARRAY;
begin
	process (clock)
	begin
		if rising_edge(clock) then
			dout <= ram(to_integer(unsigned(address)));
			if cs and rw then
				dout_valid <= '1';
			else
				dout_valid <= '0';
			end if;
			if cs then
				if not rw then
					ram(to_integer(unsigned(address))) <= din;
				end if;
			end if;
		end if;
	end process;
end Behavioral;
