library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity tmds_encoderb is
    port (
        clk  : in  std_logic;
        sel: in std_logic_vector(1 downto 0);
        ctrl : in  std_logic_vector(1 downto 0);
		aux  : in std_logic_vector(3 downto 0);
        din  : in  std_logic_vector(7 downto 0);
        dout : out std_logic_vector(9 downto 0)
    );
end tmds_encoderb;

architecture rtl of tmds_encoderb is
    signal n_ones_din : integer range 0 to 8;

    signal xored, xnored : std_logic_vector(8 downto 0);
    signal q_m : std_logic_vector(8 downto 0);

    -- a positive value represents the excess number of 1's that have been transmitted
    -- a negative value represents the excess number of 0's that have been transmitted
    signal disparity : signed(3 downto 0) := to_signed(0, 4);
    -- difference between 1's and 0's (/2 since the last bit is never used)
    signal diff : signed(3 downto 0) := to_signed(0, 4);

begin

    -- ones counter for input data
    process(din) is
        variable c : integer range 0 to 8;
    begin
        c := 0;
        for i in 0 to 7 loop
            if din(i) = '1' then
                c := c + 1;
            end if;
        end loop;
        n_ones_din <= c;
    end process;

    -- create xor encodings
    xored(0) <= din(0);
    encode_xor: for i in 1 to 7 generate
    begin
        xored(i) <= din(i) xor xored(i - 1);
    end generate;
    xored(8) <= '1';

    -- create xnor encodings
    xnored(0) <= din(0);
    encode_xnor: for i in 1 to 7 generate
    begin
        xnored(i) <= din(i) xnor xnored(i - 1);
    end generate;
    xnored(8) <= '0';

    -- use xnored or xored data based on the ones
    q_m <= xnored when n_ones_din > 4 or (n_ones_din = 4 and din(0) = '0') else xored;

    -- ones counter for internal data
    process(q_m) is
        variable c : integer range 0 to 8;
    begin
        c := 0;
        for i in 0 to 7 loop
            if q_m(i) = '1' then
                c := c + 1;
            end if;
        end loop;
        diff <= to_signed(c-4, 4);
    end process;

    process(clk) is
    begin
        if rising_edge(clk) then
            case sel is
				when "01" =>
					case ctrl is
						when "00"   => dout <= "1101010100";
						when "01"   => dout <= "0010101011";
						when "10"   => dout <= "0101010100";
						when others => dout <= "1010101011";
					end case;
					disparity <= (others => '0');
				when "00" =>
					if disparity = 0 or diff = 0 then
						-- xnored data
						if q_m(8) = '0' then
							dout <= "10" & not q_m(7 downto 0);
							disparity <= disparity - diff;
						-- xored data
						else
							dout <= "01" & q_m(7 downto 0);
							disparity <= disparity + diff;
						end if;
					elsif (diff(diff'left) = '0' and disparity(disparity'left) = '0') or
						  (diff(diff'left) = '1' and disparity(disparity'left) = '1') then
						dout <= '1' & q_m(8) & not q_m(7 downto 0);
						if q_m(8) = '1' then
							disparity <= disparity + 1 - diff;
						else
							disparity <= disparity - diff;
						end if;
					else
						dout <= '0' & q_m;
						if q_m(8) = '1' then
							disparity <= disparity + diff;
						else
							disparity <= disparity - 1 + diff;
						end if;
					end if;
				when "10" =>
					disparity <= (others => '0');
					case aux is
						when "0000" => dout <= "1010011100";
						when "0001" => dout <= "1001100011";
						when "0010" => dout <= "1011100100";
						when "0011" => dout <= "1011100010";
						when "0100" => dout <= "0101110001";
						when "0101" => dout <= "0100011110";
						when "0110" => dout <= "0110001110";
						when "0111" => dout <= "0100111100";
						when "1000" => dout <= "1011001100";
						when "1001" => dout <= "0100111001";
						when "1010" => dout <= "0110011100";
						when "1011" => dout <= "1011000110";
						when "1100" => dout <= "1010001110";
						when "1101" => dout <= "1001110001";
						when "1110" => dout <= "0101100011";
						when others => dout <= "1011000011";
					end case;
				when others => dout <= "1111100000";
             end case;
        end if;
    end process;
end rtl;

library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity tmds_multiplexer is
	Port(
		clock: in std_logic;
		pixel_clock: in std_logic;
		reset: in std_logic;
		din: in std_logic_vector(9 downto 0);
		dout: out std_logic_vector(1 downto 0));
end tmds_multiplexer;

architecture Behavioral of tmds_multiplexer is
signal counter: std_logic_vector(2 downto 0) := "000";
begin

	process (all)
	begin
		case counter is
			when "000" => dout <= din(1 downto 0);
			when "001" => dout <= din(3 downto 2);
			when "010" => dout <= din(5 downto 4);
			when "011" => dout <= din(7 downto 6);
			when others => dout <= din(9 downto 8);
		end case;
	end process;

	process (reset, clock)
	begin
		if rising_edge(clock) then
			if reset then
				counter <= "000";
			else
				counter <= std_logic_vector(unsigned(counter) + 1);
				if counter = "100" then
					counter <= "000";
				end if;
			end if;
		end if;
	end process;
end Behavioral;

library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity hdmi2 is
	Generic(h: integer := 1920;
        v: integer := 1080;
        hblank_width: integer := 280;
        vblank_width: integer := 45;
		  hsync_polarity: std_logic := '1';
		  hsync_porch: integer := 148;
		  hsync_width: integer := 44;
		  vsync_width: integer := 5;
		  vsync_porch: integer := 36;
		  vsync_polarity: std_logic := '1';
        rate: integer := 60);
	Port(
		reset: in std_logic;
		clock_freq: out integer;
		pixel_clock: in std_logic;
		tmds_clock: in std_logic;
		tmds_0: out std_logic_vector(9 downto 0);
		tmds_1: out std_logic_vector(9 downto 0);
		tmds_2: out std_logic_vector(9 downto 0);
		ck_p: out std_logic;
		ck_n: out std_logic;
		cec: inout std_logic;
		i2c_scl: inout std_logic;
		i2c_sda: inout std_logic;
		hpd: inout std_logic;
		test: out std_logic_vector(1 downto 0);
		hstart: out std_logic;
		vstart: out std_logic;
		row_out: out std_logic_vector(9 downto 0);
		column_out: out std_logic_vector(10 downto 0);
		r: in std_logic_vector(7 downto 0);
		g: in std_logic_vector(7 downto 0);
		b: in std_logic_vector(7 downto 0));
end hdmi2;

architecture Behavioral of hdmi2 is
	signal control: std_logic_vector(3 downto 0) := (others => '0');
	signal aux: std_logic_vector(3 downto 0) := (others => '0');
	signal aux2: std_logic_vector(3 downto 0) := (others => '0');
	signal aux3: std_logic_vector(3 downto 0) := (others => '0');
	
	signal tmds_0_pre: std_logic_vector(9 downto 0);
	signal tmds_1_pre: std_logic_vector(9 downto 0);
	signal tmds_2_pre: std_logic_vector(9 downto 0);
	
	signal htotal: integer := (h + hblank_width);
	signal vtotal: integer := (v + vblank_width);
	
	signal column: integer range 0 to h+hblank_width-1 := 0;
	signal row: integer range 0 to v+vblank_width-1:= 0;
	
	signal hblank: std_logic;
	signal vblank: std_logic;
	signal hsync: std_logic;
	signal hsync2: std_logic;
	signal hsync3: std_logic;
	signal hsync4: std_logic;
	signal vsync: std_logic;
	signal control_period: std_logic;
	signal pixels_guard: std_logic;
	signal pixels_guard2: std_logic;
	signal pixels: std_logic;
	signal data_island_guard: std_logic;
	signal data_island_guard2: std_logic;
	signal data_island: std_logic;
	signal data_island_preamble: std_logic;
	signal pixel_preamble: std_logic;
	
	signal request_data_island: std_logic;
	signal data_island_mode: std_logic_vector(2 downto 0) := (others => '0');
	signal data_island_guard_count: std_logic := '0';
	signal data_island_counter: std_logic_vector(9 downto 0) := (others => '0');
	
	signal control0: std_logic_vector(1 downto 0);
	signal control1: std_logic_vector(1 downto 0);
	signal control2: std_logic_vector(1 downto 0);
	
	signal selection: std_logic_vector(1 downto 0);
begin
    clock_freq <= htotal * vtotal * rate;

	test <= vsync & pixels_guard;

	data_island <= '0'; --todo
	data_island_guard <= '0';--data_island_mode(1);
	 data_island_preamble <= '0';--data_island_mode(2);
	 process (pixel_clock)
	 begin
		if rising_edge(pixel_clock) then
			case data_island_mode is
				when "000" =>
					if request_data_island then
						data_island_mode <= "100";
					end if;
				when "100" =>
					data_island_counter <= std_logic_vector(unsigned(data_island_counter) + 1);
					if data_island_counter(4 downto 0) = "00111" then
						data_island_mode <= "010";
						data_island_counter <= (others => '0');
					end if;
				when "010" =>
					data_island_guard_count <= not data_island_guard_count;
					if data_island_guard_count then
						data_island_mode <= "001";
					end if;
				when "011" =>
					data_island_guard_count <= not data_island_guard_count;
					if data_island_guard_count then
						data_island_mode <= "000";
					end if;
				when others =>
					data_island_counter <= std_logic_vector(unsigned(data_island_counter) + 1);
					if data_island_counter(4 downto 0) = "11111" then
						data_island_mode <= "011";
						data_island_counter <= (others => '0');
					end if;
			end case;
		end if;
	 end process;

	process (all)
	begin
		if data_island then
			selection <= "10";  --aux output
		elsif pixels_guard or hblank or vblank then
			selection <= "01";  --ctl output
		elsif pixels then
			selection <= "00";  --normal pixel output
		else
			selection <= "11";
		end if;
		if data_island_preamble then
			control <= "0101";
		elsif pixel_preamble then
			control <= "0001";
		else
			control <= "0000";
		end if;
	end process;
	  
	 process (pixel_clock)
	 begin
		if rising_edge(pixel_clock) then
			if column = 15 then
				request_data_island <= '0';
			else
				request_data_island <= '0';
			end if;
			if column = (hsync_porch + hsync_width - 1) then
				hstart <= '1' and not vblank;
			else
				hstart <= '0';
			end if;
			if row = (vsync_porch + vsync_width + 1) then
				if column = (hsync_porch + hsync_width - 1) then
					vstart <= '1';
				else
					vstart <= '0';
				end if;
			else
				vstart <= '0';
			end if;
			row_out <= std_logic_vector(to_unsigned(row - vsync_width - vsync_porch - 1, 10));
			column_out <= std_logic_vector(to_unsigned(column - hsync_width - hsync_porch + 1, 11));
			if column = (hsync_porch + hsync_width - 1) or column = (hsync_porch + hsync_width-2) then
				pixels_guard <= '1' and not vblank;
			else
				pixels_guard <= '0';
			end if;
			pixels_guard2 <= pixels_guard;
			data_island_guard2 <= data_island_guard;
			
			if column >= (hsync_porch + hsync_width-10) and column <= (hsync_porch + hsync_width - 3) then
				pixel_preamble <= '1' and not vblank;
			else
				pixel_preamble <= '0';
			end if;
			if column < (hsync_width + hsync_porch) or column >= (hsync_width + hsync_porch + h) then
				hblank <= '1';
			else
				hblank <= '0';
			end if;
			if row < (1 + vsync_width + vsync_porch) or row >= (1 + vsync_width + vsync_porch + v) then
				vblank <= '1';
			else
				vblank <= '0';
			end if;
			if column > 0 and column < hsync_width + 1 then
				hsync <= hsync_polarity;
			else
				hsync <= not hsync_polarity;
			end if;
			if row > 0 and row < vsync_width + 1 then
				vsync <= vsync_polarity;
			else
				vsync <= not vsync_polarity;
			end if;
			if column >= (hsync_porch + hsync_width) and column < (hsync_porch + hsync_width + h) then
				pixels <= '1' and not vblank;
			else
				pixels <= '0';
			end if;
			if column = (htotal - 1) then
				column <= 0;
				if row = (vtotal - 1) then
					row <= 0;
				else
					row <= row + 1;
				end if;
			else
				column <= column + 1;
			end if;
		end if;
	 end process;

	control0(0) <= hsync;
	control0(1) <= vsync;
	control1(0) <= control(0);
	control1(1) <= control(1);
	control2(0) <= control(2);
	control2(1) <= control(3);
	 
	d0_encoder: entity work.tmds_encoderb port map(
		clk => pixel_clock,
		din => b,
		ctrl => control0,
		aux => aux,
		sel => selection,
		dout => tmds_0_pre
	);
	
	process (pixel_clock)
	begin
        if rising_edge(pixel_clock) then
            if pixels_guard2 then
                tmds_0 <= "1011001100";
                tmds_1 <= "0100110011";
                tmds_2 <= "1011001100";
            elsif data_island_guard2 then
                tmds_0 <= tmds_0_pre;
                tmds_1 <= "0100110011";
                tmds_2 <= "0100110011";
            else
                tmds_0 <= tmds_0_pre;
                tmds_1 <= tmds_1_pre;
                tmds_2 <= tmds_2_pre;
            end if;
        end if;
	end process;
	
	d1_encoder: entity work.tmds_encoderb port map(
		clk => pixel_clock,
		din => g,
		ctrl => control1,
		aux => aux2,
		sel => selection,
		dout => tmds_1_pre
	);
	
	d2_encoder: entity work.tmds_encoderb port map(
		clk => pixel_clock,
		din => r,
		ctrl => control2,
		aux => aux3,
		sel => selection,
		dout => tmds_2_pre
	);
	
	ck_p <= pixel_clock;
	ck_n <= not pixel_clock;

	process(pixel_clock)
	begin
		if rising_edge(pixel_clock) then	
		end if;
	end process;
end Behavioral;