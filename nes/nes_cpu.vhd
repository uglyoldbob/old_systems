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
--use IEEE.NUMERIC_STD.ALL;

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
	signal s: std_logic_vector(7 downto 0);
	signal p: std_logic_vector(5 downto 0);
begin


end Behavioral;

