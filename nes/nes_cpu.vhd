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
	signal a: std_logic_vector(7 downto 0);
	signal x: std_logic_vector(7 downto 0);
	signal y: std_logic_vector(7 downto 0);
	signal pc: std_logic_vector(15 downto 0);
	signal sp: std_logic_vector(7 downto 0);
	signal flags: std_logic_vector(7 downto 0);
	
	signal clock0: std_logic;
	signal clock1: std_logic;
	signal clock2: std_logic;
	signal clock_divider: std_logic_vector(3 downto 0);
	
	signal reset_vector: std_logic;
	signal instruction_cycle: std_logic_vector(4 downto 0);
	signal calculated_addr: std_logic_vector(15 downto 0);
	
	signal cycle_number: std_logic_vector(19 downto 0);
	
	signal data_in: std_logic_vector(7 downto 0);
begin
	m2 <= clock2;
	process (reset, clock, clock0, clock_divider)
	begin
		if reset='0' then
			clock_divider <= (others =>'0');
			clock0 <= '0';
			clock1 <= '0';
			clock2 <= '0';
		elsif rising_edge(clock) then
			if clock_divider = "0101" then
				clock_divider <= (others =>'0');
				clock0 <= not clock0;
			else
				clock_divider <= std_logic_vector(unsigned(clock_divider) + to_unsigned(1, clock_divider'length));
			end if;
			clock1 <= not clock0;
			clock2 <= clock0;
		end if;
	end process;
	
	process (reset, clock2)
	begin
		if reset='0' then
			pc <= x"FFFE";
		elsif rising_edge(clock2) then
			data_in <= data;
			if reset_vector='1' then
				case instruction_cycle is
					when "00110" =>
						pc(7 downto 0) <= data;
					when "00111" =>
						pc(15 downto 8) <= data;
					when others =>
						null;
				end case;
			end if;
		end if;
	end process;
	
	process (reset, clock1)
	begin
		if reset='0' then
			reset_vector <= '1';
			address <= (others => 'Z');
			data <= (others => 'Z');
			instruction_cycle <= (others => '0');
			cycle_number <= (others => '0');
			calculated_addr <= "1010101010101010";
			sp <= "10001000";
		elsif rising_edge(clock1) then
			cycle_number <= std_logic_vector(unsigned(cycle_number) + to_unsigned(1, cycle_number'length));
			if reset_vector='1' then
				rw <= '1';
				case instruction_cycle is
					when "00000" =>
						address <= calculated_addr;
						calculated_addr <= std_logic_vector(unsigned(calculated_addr) + to_unsigned(1,16));
						instruction_cycle <= std_logic_vector(unsigned(instruction_cycle) + to_unsigned(1,instruction_cycle'length));
					when "00001" =>
						address <= calculated_addr;
						instruction_cycle <= std_logic_vector(unsigned(instruction_cycle) + to_unsigned(1,instruction_cycle'length));
					when "00010" =>
						address <= std_logic_vector(unsigned(sp) + to_unsigned(256,16));
						instruction_cycle <= std_logic_vector(unsigned(instruction_cycle) + to_unsigned(1,instruction_cycle'length));
					when "00011" =>
						address <= std_logic_vector(unsigned(sp) + to_unsigned(255,16));
						instruction_cycle <= std_logic_vector(unsigned(instruction_cycle) + to_unsigned(1,instruction_cycle'length));
					when "00100" =>
						address <= std_logic_vector(unsigned(sp) + to_unsigned(254,16));
						instruction_cycle <= std_logic_vector(unsigned(instruction_cycle) + to_unsigned(1,instruction_cycle'length));
					when "00101" =>
						address <= x"FFFC";
						instruction_cycle <= std_logic_vector(unsigned(instruction_cycle) + to_unsigned(1,instruction_cycle'length));
					when "00110" =>
						address <= x"FFFD";
						instruction_cycle <= std_logic_vector(unsigned(instruction_cycle) + to_unsigned(1,instruction_cycle'length));
					when others =>
						address <= pc;
						reset_vector <= '0';
				end case;
			end if;
		end if;
	end process;

end Behavioral;

