library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use ieee.std_logic_misc.all;

use IEEE.NUMERIC_STD.ALL;

entity clock_divider is
	Port (
		reset: in std_logic;
		clock: in std_logic;
		c1: out std_logic;
		c2: out std_logic;
		c3: out std_logic);
end clock_divider;

architecture Behavioral of clock_divider is
	signal counter: std_logic_vector(2 downto 0);
	signal clocko: std_logic;
	signal clocko2: std_logic;
begin
	c1 <= clocko;
	c2 <= clocko2;
	process (reset, clock)
	begin
		if reset='1' then
			counter <= "000";
			clocko <= '0';
			clocko2 <= '1';
		elsif rising_edge(clock) then
			c3 <= clocko;
			counter <= std_logic_vector(unsigned(counter(2 downto 0)) + "001");
			if counter = "110" then
				counter <= "000";
				clocko <= not clocko;
				clocko2 <= not clocko2;
			end if;
		end if;
	end process;
end Behavioral;

library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use ieee.std_logic_misc.all;

use IEEE.NUMERIC_STD.ALL;

entity nes_cpu is
    Port ( clock : in STD_LOGIC;
           audio : out STD_LOGIC_VECTOR (1 downto 0);
           address : out STD_LOGIC_VECTOR (15 downto 0);
			  memory_start: out std_logic;
			  memory_clock: out std_logic;
			  memory_cycle_done: in std_logic;
           dout : out STD_LOGIC_VECTOR (7 downto 0);
			  din: in STD_LOGIC_VECTOR (7 downto 0);
           cout : out STD_LOGIC_VECTOR (2 downto 0);
			  oe: out STD_LOGIC_VECTOR (1 downto 0);
           rw : out STD_LOGIC;
           nmi : in STD_LOGIC;
           irq : in STD_LOGIC;
           m2 : out STD_LOGIC;
           tst : in STD_LOGIC;
           reset : in STD_LOGIC);
end nes_cpu;

architecture Behavioral of nes_cpu is
	signal clocka: std_logic;
	signal clockb: std_logic;
	signal clockm: std_logic;

	signal a: std_logic_vector(7 downto 0);
	signal x: std_logic_vector(7 downto 0);
	signal y: std_logic_vector(7 downto 0);
	signal pc: std_logic_vector(15 downto 0);
	signal sp: std_logic_vector(7 downto 0);
	signal flags: std_logic_vector(7 downto 0);
	
	signal next_pc: std_logic_vector(15 downto 0);
	
	constant FLAG_CARRY: integer := 0;
	constant FLAG_ZERO: integer := 1;
	constant FLAG_INTERRUPT: integer := 2;
	constant FLAG_DECIMAL: integer := 3;
	constant FLAG_BREAK: integer := 4;
	constant FLAG_UNUSED: integer := 5;
	constant FLAG_OVERFLOW: integer := 6;
	constant FLAG_NEGATIVE: integer := 7;
	
	signal reset_active: std_logic; --Indicates that the cpu is in reset
	signal opcode: std_logic_vector(8 downto 0); -- bit 8 indicates the opcode is valid
	signal dma_cycle: std_logic;
	
	signal subcycle: std_logic_vector(3 downto 0);
	signal oe1: std_logic;
	signal oe2: std_logic;
	
	signal dma_running: std_logic;
	signal dma_dmc: std_logic_vector(16 downto 0);
	signal dma_oam: std_logic_vector(8 downto 0);
	signal dma_count: std_logic_vector(8 downto 0);
	signal dma_dmc_counter: std_logic_vector(7 downto 0);
	
	signal rw_address: std_logic_vector(15 downto 0);
	signal read_cycle: std_logic; --indicates a read cycle when true
	signal stall: std_logic;
	signal stall_clocked: std_logic;
	
	signal instruction_length: std_logic_vector(1 downto 0);
begin

	clockd: entity work.clock_divider port map (
		reset => reset,
		clock => clock,
		c1 => clocka,
		c2 => clockb,
		c3 => clockm
		);

	memory_clock <= clockm;
		
   process (rw_address)
	begin
		if rw_address = x"4016" then
			oe1 <= '0';
		else
			oe1 <= '1';
		end if;
	end process;

	process (rw_address)
	begin
		if rw_address = x"4017" then
			oe2 <= '0';
		else
			oe2 <= '1';
		end if;
	end process;
	oe <= oe1 & oe2;
	
	process (all)
	begin
		if read_cycle = '1' then
			stall <= '0';
			if dma_running then
				rw <= '1';
			else
				address <= rw_address;
				rw <= '1';
				if dma_dmc(16) = '1' then
					dma_running <= '1';
				elsif dma_oam(8) = '1' then
					dma_running <= '1';
				end if;
			end if;
		else
			stall <= '0';
			rw <= '0';
			address <= rw_address;
		end if;
	end process;

	process (reset, clocka)
	begin
		if reset='1' then
			sp <= std_logic_vector(unsigned(sp(7 downto 0)) - "00000011");
			flags <= flags OR "00000100";
			read_cycle <= '1';
			memory_start <= '0';
		elsif rising_edge(clocka) then
			dma_cycle <= NOT dma_cycle;
			if reset_active then
				read_cycle <= '1';
				next_pc <= pc;
				case subcycle is
					when "0000" =>
						rw_address <= pc;
						next_pc <= std_logic_vector(unsigned(pc(15 downto 0)) + "0000000000000001");
					when "0001" =>
						rw_address <= pc;
						next_pc <= std_logic_vector(unsigned(pc(15 downto 0)) + "0000000000000001");
					when "0010" =>
						rw_address <= std_logic_vector(unsigned(sp(7 downto 0)) + x"0100");
					when "0011" =>
						rw_address <= std_logic_vector(unsigned(sp(7 downto 0)) + x"00ff");
					when "0100" =>
						rw_address <= std_logic_vector(unsigned(sp(7 downto 0)) + x"00fe");
					when "0101" =>
						rw_address <= x"FFFC";
					when "0110" =>
						rw_address <= x"FFFD";
					when others =>
					   rw_address <= "XXXXXXXXXXXXXXXX";
				end case;
			else
				if opcode(8) = '0' then
					rw_address <= pc;
					next_pc <= std_logic_vector(unsigned(pc(15 downto 0)) + "0000000000000001");
					read_cycle <= '1';
				else
				end if;
			end if;
		end if;
	end process;
	
	process (all)
	begin
		case opcode(7 downto 0) is
			when x"01" => instruction_length <= "01";
			when others => instruction_length <= "01";
		end case;
	end process;
	
	process (clockm)
	begin
		if rising_edge(clockm) then
			stall_clocked <= stall;
		end if;
	end process;
	
	process(reset, clockb)
	begin
		if reset='1' then
			pc <= x"FFFC";
			opcode <= "011111111";
			reset_active <= '1';
			subcycle <= "0000";
		elsif rising_edge(clockb) then
			if reset_active then
				subcycle <= std_logic_vector(unsigned(subcycle(3 downto 0)) + "0001");
				pc <= next_pc;
				case subcycle(2 downto 0) is
					when "000" =>
					when "001" =>
					when "010" =>
					when "011" =>
					when "100" =>
					when "101" =>
						pc <= next_pc(15 downto 8) & din;
					when others =>
						pc <= din & next_pc(7 downto 0);
						reset_active <= '0';
						subcycle <= (others => '0');
				end case;
			elsif stall_clocked = '0' then
				pc <= next_pc;
				if opcode(8) = '0' then
					opcode <= '1' & din;
				else
					case opcode(7 downto 0) is
						when x"00" =>
						when others =>
					end case;
				end if;
			end if;
		end if;
	end process;

end Behavioral;

