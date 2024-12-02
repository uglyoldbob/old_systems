library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity sram is
	generic (bits: integer := 15;
				dbits: integer := 8);
	port ( addr : in  STD_LOGIC_VECTOR (bits-1 downto 0);
          data : inout  STD_LOGIC_VECTOR (dbits-1 downto 0);
          oe : in  STD_LOGIC;
          we : in  STD_LOGIC;
          cs : in  STD_LOGIC);
end sram;

architecture Behavioral of sram is
	type memory is array ((2**bits)-1 downto 0) of std_logic_vector(dbits-1 downto 0);
	signal storage: memory;
begin

	process (cs, oe, we, addr, data)
	begin
		if cs='0' and oe='0' then
			data <= storage(to_integer(unsigned(addr)));
		else
			data <= (others => 'Z');
		end if;
	end process;

end Behavioral;



library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;
use std.textio.all;
use ieee.std_logic_textio.all;

entity sram_init is
	generic (bits: integer := 15;
				dbits: integer = 8;
				filename: string := "none");
	port ( addr : in  STD_LOGIC_VECTOR (bits-1 downto 0);
          data : inout  STD_LOGIC_VECTOR (dbits-1 downto 0);
          oe : in  STD_LOGIC;
          we : in  STD_LOGIC;
          cs : in  STD_LOGIC;
			 whocares: out std_logic);
end sram_init;

architecture Behavioral of sram_init is
	type memory is array ((2**bits)-1 downto 0) of std_logic_vector(dbits-1 downto 0);

	impure function InitRomFromFile (RomFileName : in string) return memory is
		FILE romfile : text is in RomFileName;
		variable RomFileLine : line;
		variable rom : memory;
		begin
		for i in memory'low to memory'high loop
		readline(romfile, RomFileLine);
		hread(RomFileLine, rom(i));
		end loop;
		return rom;
	end function;

	signal storage: memory := InitRomFromFile(filename);
	
begin

	process (cs, oe, we, addr, data)
	begin
		if cs='0' and oe='0' then
			data <= storage(to_integer(unsigned(addr)));
			whocares <= '1';
		else
			data <= (others => 'Z');
			whocares <= '0';
		end if;
	end process;

end Behavioral;

