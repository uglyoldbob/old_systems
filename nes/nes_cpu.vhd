----------------------------------------------------------------------------------
-- Company: 
-- Engineer: 
-- 
-- Create Date:    20:51:09 07/27/2018 
-- Design Name: 
-- Module Name:    nes_cpu - Behavioral 
-- Project Name: 
-- Target Devices: 
-- Tool versions: 
-- Description: 
--
-- Dependencies: 
--
-- Revision: 
-- Revision 0.01 - File Created
-- Additional Comments: 
--
----------------------------------------------------------------------------------
library IEEE;
use IEEE.STD_LOGIC_1164.ALL;

-- Uncomment the following library declaration if using
-- arithmetic functions with Signed or Unsigned values
use IEEE.NUMERIC_STD.ALL;

-- Uncomment the following library declaration if instantiating
-- any Xilinx primitives in this code.
--library UNISIM;
--use UNISIM.VComponents.all;

entity nes_cpu is
    Port ( clock : in  STD_LOGIC;
           audio : out  STD_LOGIC_VECTOR (1 downto 0);
           address : out  STD_LOGIC_VECTOR (15 downto 0);
           data : inout  STD_LOGIC_VECTOR (7 downto 0);
           cout : out  STD_LOGIC_VECTOR (2 downto 0);
           rw : out  STD_LOGIC;
           nmi : in  STD_LOGIC;
           irq : in  STD_LOGIC;
           m2 : out  STD_LOGIC;
           tst : in  STD_LOGIC;
           reset : in  STD_LOGIC);
end nes_cpu;

architecture Behavioral of nes_cpu is
	type memory is array (7 downto 0) of std_logic_vector(7 downto 0);

	signal a: std_logic_vector(7 downto 0);
	signal x: std_logic_vector(7 downto 0);
	signal y: std_logic_vector(7 downto 0);
	signal pc: std_logic_vector(15 downto 0);
	signal sp: std_logic_vector(7 downto 0);
	signal flags: std_logic_vector(7 downto 0);
	
	constant FLAG_CARRY: integer := 0;
	constant FLAG_ZERO: integer := 1;
	constant FLAG_INTERRUPT: integer := 2;
	constant FLAG_DECIMAL: integer := 3;
	constant FLAG_BREAK: integer := 4;
	constant FLAG_UNUSED: integer := 5;
	constant FLAG_OVERFLOW: integer := 6;
	constant FLAG_NEGATIVE: integer := 7;
	
	signal clock1: std_logic;	--clock1 toggles on the falling edge of the input clock, every 6 clock cycles
	signal clock2: std_logic;
	
	signal johnson_divider: std_logic_vector(6 downto 0);
	
	signal reset_vector: std_logic;
	signal instruction_cycle: std_logic_vector(4 downto 0);
	signal calculated_addr: std_logic_vector(15 downto 0);
	
	signal cycle_number: std_logic_vector(19 downto 0);
	
	signal data_in: std_logic_vector(7 downto 0);
	
	signal executing_instruction: memory;
	
	signal opcode_bytes: std_logic_vector(2 downto 0); --calculation of (number of bytes - 1) to read for the current opcode
	signal opcode_cycles: std_logic_vector(2 downto 0); --calculation of number of execution cycles for the current opcode
begin
	m2 <= johnson_divider(6) or johnson_divider(0);
	clock1 <= not johnson_divider(6);
	clock2 <= johnson_divider(6);
	process (reset, clock)
	begin
		if reset='0' then
			johnson_divider <= (others => '0');
		elsif rising_edge(clock) then
			johnson_divider(0) <= not johnson_divider(5);
			johnson_divider(1) <= johnson_divider(0);
			johnson_divider(2) <= johnson_divider(1);
			johnson_divider(3) <= johnson_divider(2);
			johnson_divider(4) <= johnson_divider(3);
			johnson_divider(5) <= johnson_divider(4);
		end if;
		if falling_edge(clock) then
			johnson_divider(6) <= johnson_divider(1);
		end if;
	end process;
	
	--registers process
	process (reset, clock2)
	begin
		if reset='0' then
			pc <= x"FFFE";
			sp <= "10001000";
			reset_vector <= '1';
			calculated_addr <= "1010101010101010";
			instruction_cycle <= (others => '0');
		elsif rising_edge(clock2) then
			if reset_vector='1' then
				instruction_cycle <= std_logic_vector(unsigned(instruction_cycle) + to_unsigned(1,instruction_cycle'length));
				case instruction_cycle is
					when "00000" =>
						null;
					when "00001" =>
						calculated_addr <= std_logic_vector(unsigned(calculated_addr) + to_unsigned(1,16));
					when "00010" =>
						calculated_addr <= std_logic_vector(unsigned(sp) + to_unsigned(256,16));
					when "00011" =>
						calculated_addr <= std_logic_vector(unsigned(sp) + to_unsigned(255,16));
					when "00100" =>
						calculated_addr <= std_logic_vector(unsigned(sp) + to_unsigned(254,16));
					when "00101" =>
						calculated_addr <= x"FFFC";
					when "00110" =>
						pc(7 downto 0) <= data;
						calculated_addr <= x"FFFD";
					when others =>
						pc(15 downto 8) <= data;
						calculated_addr <= data & pc(7 downto 0);
						instruction_cycle <= (others => '0');
						reset_vector <= '0';
				end case;
			else
				case instruction_cycle is
					when "00000" =>
						pc <= std_logic_vector(unsigned(pc) + to_unsigned(1,16));
						
					when others =>
						null;
				end case;
			end if;
		end if;
	end process;
	
	process (reset, clock2)
	begin
		if reset='1' and rising_edge(clock2) then
			if reset_vector='0' then
				case instruction_cycle is
					when "00000" =>
						case data is
							when x"0a" | x"10" | x"29" | x"30" | x"50" | x"69" | x"90" | x"b0" | x"d0" | x"f0" =>
								opcode_cycles <= "001";
							when x"24" | x"25" | x"4c" | x"65" =>
								opcode_cycles <= "010";
							when x"2c" | x"2d" | x"35" | x"39" | x"3d" | x"6d" | x"75" | x"79" | x"7d" =>
								opcode_cycles <= "011";
							when x"06" | x"31" | x"71" =>
								opcode_cycles <= "100";
							when x"0e" | x"16" | x"21" | x"61" =>
								opcode_cycles <= "101";
							when x"00" | x"1e" =>
								opcode_cycles <= "110";
							when others => 
								opcode_cycles <= "111";
						end case;
					when others =>
						null;
				end case;
			end if;
		end if;
	end process;

	process (reset, clock1)
	begin
		if reset='0' then
			address <= (others => 'Z');
			data <= (others => 'Z');
			cycle_number <= (others => '0');
		elsif rising_edge(clock1) then
			cycle_number <= std_logic_vector(unsigned(cycle_number) + to_unsigned(1, cycle_number'length));
			address <= calculated_addr;
			if reset_vector='1' then
				rw <= '1';
			else
				null;
			end if;
		end if;
	end process;

end Behavioral;

