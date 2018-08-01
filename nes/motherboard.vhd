----------------------------------------------------------------------------------
-- Company: 
-- Engineer: 
-- 
-- Create Date:    20:44:49 07/16/2018 
-- Design Name: 
-- Module Name:    motherboard - Behavioral 
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

entity motherboard is
    Port (whocares: out std_logic;
		otherstuff: out std_logic_vector(15 downto 0));
end motherboard;

architecture Behavioral of motherboard is
	signal cpu_clock: std_logic := '0';
	signal cpu_reset: std_logic := '0';
	signal cpu_address: std_logic_vector(15 downto 0);
begin
	whocares <= cpu_clock;
	otherstuff <= cpu_address;
	cpu: entity work.nes_cpu port map (
		clock => cpu_clock, 
		reset => cpu_reset,
		nmi => '1',
		irq => '1',
		tst => '0',
		address => cpu_address);

end Behavioral;

