--------------------------------------------------------------------------------
-- Company: 
-- Engineer:
--
-- Create Date:   21:36:58 07/27/2018
-- Design Name:   
-- Module Name:   E:/software/cpu/nes/nes_test.vhd
-- Project Name:  nes
-- Target Device:  
-- Tool versions:  
-- Description:   
-- 
-- VHDL Test Bench Created by ISE for module: nes_cpu
-- 
-- Dependencies:
-- 
-- Revision:
-- Revision 0.01 - File Created
-- Additional Comments:
--
-- Notes: 
-- This testbench has been automatically generated using types std_logic and
-- std_logic_vector for the ports of the unit under test.  Xilinx recommends
-- that these types always be used for the top-level I/O of a design in order
-- to guarantee that the testbench will bind correctly to the post-implementation 
-- simulation model.
--------------------------------------------------------------------------------
LIBRARY ieee;
USE ieee.std_logic_1164.ALL;
 
-- Uncomment the following library declaration if using
-- arithmetic functions with Signed or Unsigned values
--USE ieee.numeric_std.ALL;
 
ENTITY nes_test IS
END nes_test;
 
ARCHITECTURE behavior OF nes_test IS 
 
    -- Component Declaration for the Unit Under Test (UUT)
 
    COMPONENT nes_cpu
    PORT(
         clock : IN  std_logic;
         audio : OUT  std_logic_vector(1 downto 0);
         address : OUT  std_logic_vector(15 downto 0);
         data : INOUT  std_logic_vector(7 downto 0);
         cout : OUT  std_logic_vector(2 downto 0);
         rw : OUT  std_logic;
         nmi : IN  std_logic;
         irq : IN  std_logic;
         m2 : OUT  std_logic;
         tst : IN  std_logic;
         reset : IN  std_logic
        );
    END COMPONENT;

	type memory is array (2047 downto 0 of std_logic_vector(7 downto 0);

	signal mb_ram: memory;

   --Inputs
   signal clock : std_logic := '0';
   signal nmi : std_logic := '0';
   signal irq : std_logic := '0';
   signal tst : std_logic := '0';
   signal reset : std_logic := '0';

	--BiDirs
   signal data : std_logic_vector(7 downto 0);

 	--Outputs
   signal audio : std_logic_vector(1 downto 0);
   signal address : std_logic_vector(15 downto 0);
   signal cout : std_logic_vector(2 downto 0);
   signal rw : std_logic;
   signal m2 : std_logic;

   -- Clock period definitions
   constant clock_period : time := 46.56 ns;
 
BEGIN
 
	-- Instantiate the Unit Under Test (UUT)
   uut: nes_cpu PORT MAP (
          clock => clock,
          audio => audio,
          address => address,
          data => data,
          cout => cout,
          rw => rw,
          nmi => nmi,
          irq => irq,
          m2 => m2,
          tst => tst,
          reset => reset
        );

   -- Clock process definitions
   clock_process :process
   begin
		clock <= '0';
		wait for clock_period/2;
		clock <= '1';
		wait for clock_period/2;
   end process;
 

   -- Stimulus process
   stim_proc: process
   begin		
      -- hold reset state for 100 ns.
      wait for 100 ns;	
		reset <= '1';
      wait for clock_period*10;

      -- insert stimulus here 

      wait;
   end process;

END;
