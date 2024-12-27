library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity nes_cartridge is
	Generic (
		ramtype: string := "sram";
        rambits: integer := 3);
   Port (
		rom_wb_ack: in std_logic;
		rom_wb_d_miso: in std_logic_vector(2**rambits-1 downto 0);
		rom_wb_d_mosi: out std_logic_vector(2**rambits-1 downto 0);
		rom_wb_err: in std_logic;
		rom_wb_addr: out std_logic_vector(25-rambits downto 0);
		rom_wb_bte: out std_logic_vector(1 downto 0);
		rom_wb_cti: out std_logic_vector(2 downto 0);
		rom_wb_cyc: out std_logic;
		rom_wb_sel: out std_logic_vector(rambits-3 downto 0);
		rom_wb_stb: out std_logic;
		rom_wb_we: out std_logic;
		ppu_data_in: in std_logic_vector(7 downto 0);
		ppu_data_out: out std_logic_vector(7 downto 0);
		ppu_addr: in std_logic_vector(13 downto 0);
		ppu_addr_a13_n: in std_logic;
		ppu_wr: in std_logic;
		ppu_rd: in std_logic;
		ciram_a10: out std_logic;
		ciram_ce: out std_logic;
		irq: out std_logic;
		cpu_rw: in std_logic;
		romsel: in std_logic;
		cpu_data_in: out std_logic_vector(7 downto 0);
		cpu_data_out: in std_logic_vector(7 downto 0);
		cpu_data_in_ready: out std_logic;
		cpu_addr: in std_logic_vector(15 downto 0);
		cpu_memory_start: in std_logic;
		m2: in std_logic;
		clock: in std_logic);
end nes_cartridge;

architecture Behavioral of nes_cartridge is
	signal mapper: std_logic_vector(15 downto 0) := x"0000";
	
	signal mirroring: std_logic;

	signal prg_rom_din: std_logic_vector(7 downto 0);
	signal prg_rom_address: std_logic_vector(21 downto 0);
	signal prg_rom_data: std_logic_vector(7 downto 0);
	signal prg_rom_data_ready: std_logic;
	signal prg_rom_cs: std_logic;
	signal prg_rom_rw: std_logic;
	
	signal chr_rom_address: std_logic_vector(12 downto 0);
	signal chr_rom_data: std_logic_vector(7 downto 0);
	signal chr_rom_cs: std_logic;
	
	signal rom_wb_mode: integer range 0 to 3 := 0;
begin
	process (all)
	begin
		if ramtype="wishbone" then
            if rambits = 4 then
                rom_wb_sel <= cpu_addr(0) & not cpu_addr(0);
                rom_wb_d_mosi <= cpu_data_out & cpu_data_out;
                if cpu_addr(0) then
                    cpu_data_in <= rom_wb_d_miso(15 downto 8);
                else
                    cpu_data_in <= rom_wb_d_miso(7 downto 0);
                end if;
                rom_wb_addr <= "0" & prg_rom_address(21 downto 1);
            else
                rom_wb_sel <= "1";
                rom_wb_d_mosi <= cpu_data_out;
                cpu_data_in <= rom_wb_d_miso;
                rom_wb_addr <= "0" & prg_rom_address(21 downto 0);
            end if;
			rom_wb_we <= '1';
			
			rom_wb_bte <= "00";
			rom_wb_cti <= "000";
		end if;
	
		chr_rom_cs <= '0';
		case mapper is
			when x"0000" =>
				prg_rom_address(21 downto 15) <= (others => '0');
				prg_rom_address(14 downto 0) <= cpu_addr(14 downto 0);
				if cpu_addr(15) = '1' then
					prg_rom_cs <= '1';
				else
					prg_rom_cs <= '0';
				end if;
			when others =>
				prg_rom_address <= (others => '0');
				prg_rom_cs <= '0';
				cpu_data_in <= (others => '0');
		end case;
		case rom_wb_mode is
			when 0 =>
				rom_wb_cyc <= '0';
				rom_wb_stb <= '0';
				cpu_data_in_ready <= '1';
			when 1 =>
				rom_wb_cyc <= '1';
				rom_wb_stb <= '1';
				cpu_data_in_ready <= '0';
			when 2 =>
				rom_wb_cyc <= '1';
				rom_wb_stb <= '1';
				cpu_data_in_ready <= '1';
			when others => 
				rom_wb_cyc <= '0';
				rom_wb_stb <= '0';
				cpu_data_in_ready <= '0';
		end case;
	end process;
	
	process (clock)
	begin
		if rising_edge(clock) then
			case rom_wb_mode is
				when 0 =>
					if cpu_memory_start and prg_rom_cs then
						rom_wb_mode <= 1;
					end if;
				when 1 =>
					if rom_wb_ack then
						rom_wb_mode <= 2;
					end if;
				when 2 =>
					rom_wb_mode <= 0;
				when others => null;
			end case;
		end if;
	end process;
	
	memory: if ramtype = "sram" generate
		prg_rom: entity work.clocked_sram 
			generic map (bits => 14)
			port map(
				clock => m2,
				fast_clock => clock,
				address => prg_rom_address(13 downto 0),
				rw => prg_rom_rw,
				cs => prg_rom_cs,
				dout => prg_rom_data,
				dout_valid => prg_rom_data_ready,
				din => prg_rom_din);
		chr_rom: entity work.clocked_sram
			generic map (bits => 13)
			port map(
				clock => m2,
				address => (others => '0'),
				rw => not ppu_wr,
				cs => chr_rom_cs,
				dout => chr_rom_data,
				din => ppu_data_in);
	end generate;

end Behavioral;

