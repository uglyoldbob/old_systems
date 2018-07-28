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
    Port ( clock : in  STD_LOGIC;
			  whocares: out std_logic);
end motherboard;

architecture Behavioral of motherboard is
	signal cpu_clock: std_logic;
begin
	whocares <= clock;
	
	cpu: entity work.nes_cpu port map (clock => cpu_clock);

end Behavioral;

