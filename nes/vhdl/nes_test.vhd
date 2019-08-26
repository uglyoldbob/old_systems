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
use IEEE.NUMERIC_STD.ALL;
use ieee.std_logic_misc.all;
 
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

	signal ram_we: std_logic;
	signal ram_oe: std_logic;
	signal ram_addr: std_logic_vector(10 downto 0);
	signal ram_cs: std_logic;
	signal ram_d: std_logic_vector(7 downto 0);

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

	signal controller1_clock: std_logic;
	signal controller1_out: std_logic;
	signal controller1_d0: std_logic;
	signal controller1_d1: std_logic;
	signal controller1_d2: std_logic;
	signal controller1_d3: std_logic;
	signal controller1_d4: std_logic;
	signal controller1_cs: std_logic;
	
	signal controller2_clock: std_logic;
	signal controller2_out: std_logic;
	signal controller2_d0: std_logic;
	signal controller2_d1: std_logic;
	signal controller2_d2: std_logic;
	signal controller2_d3: std_logic;
	signal controller2_d4: std_logic;
	signal controller2_cs: std_logic;
	
	signal controller_inp0: std_logic;
	signal controller_inp1: std_logic;
	
	signal cartridge_romsel: std_logic;
	
	component famicom_cartridge_slot is
		port(ppu_data: inout std_logic_vector(7 downto 0);
			ppu_addr: in std_logic_vector(13 downto 0);
			ppu_addr_13: in std_logic;
			ppu_wr: in std_logic;
			ppu_rd: in std_logic;
			ciram_a10: out std_logic;
			ciram_ce: out std_logic;
			irq: out std_logic;
			cpu_rw: in std_logic;
			romsel: in std_logic;
			cpu_data: inout std_logic_vector(7 downto 0);
			cpu_addr: in std_logic_vector(14 downto 0);
			m2: in std_logic;
			clock: in std_logic;
			audio_in: in std_logic;
			audio_out: out std_logic
			);
	end component;

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

	mb_ram: entity work.sram 
		generic map(num_bits => 11)
		port map(
		cs => or_reduce(address(15 downto 13)),
		oe => ram_oe,
		we => ram_we,
		addr => address(10 downto 0),
		data => ram_d);

	cartridge_romsel <= address(15) nand m2;

	cartridge1: entity work.nes_cartridge port map (
		clock => clock,
		m2 => m2,
		cic_in => '0',
		cic_rst => '0',
		cic_clk => '0',
		ppu_addr => (others => '0'),
		ppu_addr_13 => '1',
		ppu_wr => '1',
		ppu_rd => '1',
		cpu_rw => rw,
		romsel => cartridge_romsel,
		cpu_data => data,
		cpu_addr => address(14 downto 0));

	process (controller1_cs)
	begin
		controller1_clock <= controller1_cs;
		controller2_clock <= controller2_cs;
		controller_inp0 <= controller1_cs;
		controller1_cs <= 'H';	--5k6 pullup
		controller2_cs <= 'H';	--5k6 pullup
		controller1_out <= cout(0);
		controller2_out <= cout(0);
		controller1_d0 <= 'H';	--10k pullup resistor
		controller1_d1 <= 'H';	--10k pullup resistor
		controller1_d2 <= 'H';  --10k pullup resistor
		controller1_d3 <= 'H';	--10k pullup resistor
		controller1_d4 <= 'H';	--10k pullup resistor
		controller2_d0 <= 'H';	--10k pullup resistor
		controller2_d1 <= 'H';	--10k pullup resistor
		controller2_d2 <= 'H';  --10k pullup resistor
		controller2_d3 <= 'H';	--10k pullup resistor
		controller2_d4 <= 'H';	--10k pullup resistor
		if controller1_cs='0' then
			data(0) <= controller1_d0;
			data(1) <= controller1_d1;
			data(2) <= controller1_d2;
			data(3) <= controller1_d3;
			data(4) <= controller1_d4;
		else
			data(0) <= 'Z';
			data(1) <= 'Z';
			data(2) <= 'Z';
			data(3) <= 'Z';
			data(4) <= 'Z';
		end if;
		if controller2_cs='0' then
			data(3) <= controller2_d3;
			data(4) <= controller2_d4;
		else
			data(3) <= 'Z';
			data(4) <= 'Z';
		end if;
		if controller_inp1='0' then
			data(0) <= controller2_d0;
			data(1) <= controller2_d1;
			data(2) <= controller2_d2;
		else
			data(0) <= 'Z';
			data(1) <= 'Z';
			data(2) <= 'Z';
		end if;
	end process;

	nmi <= 'H';
	irq <= 'H';

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
