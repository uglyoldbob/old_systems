library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity lfsr32_hpf is
    port (
        clock     : in  std_logic;
        hp_enable : in  std_logic;
        dout      : out std_logic_vector(31 downto 0)
    );
end lfsr32_hpf;

architecture rtl of lfsr32_hpf is

    -- =========================
    -- LFSR
    -- =========================
    signal d : std_logic_vector(31 downto 0) :=
        (0 => '1', others => '0');

    signal reset : std_logic;
    signal e     : std_logic;

    -- =========================
    -- FILTER STATE
    -- =========================
    signal x_in  : signed(31 downto 0);
    signal x_z1  : signed(31 downto 0) := (others => '0');
    signal y_z1  : signed(31 downto 0) := (others => '0');
    signal y_out : signed(31 downto 0);

begin

    -- =========================
    -- OUTPUT SELECT
    -- =========================
    dout <= std_logic_vector(
        y_out when hp_enable = '1' else
        signed(d)
    );

    -- =========================
    -- LFSR TAP
    -- =========================
    e <= d(31) xnor d(21) xnor d(1) xnor d(0);

    process(all)
    begin
        if d = x"00000000" then
            reset <= '1';
        else
            reset <= '0';
        end if;
    end process;

    process(clock)
    begin
        if rising_edge(clock) then
            d <= d(30 downto 0) & (reset or e);
        end if;
    end process;

    -- =========================
    -- HIGH-PASS FILTER (α = 0.5)
    -- y[n] = 0.5 * (y[n-1] + x[n] - x[n-1])
    -- =========================
    x_in <= signed(d);

    process(clock)
        variable diff : signed(32 downto 0);
        variable sum  : signed(32 downto 0);
        variable ytmp : signed(32 downto 0);
    begin
        if rising_edge(clock) then

            if hp_enable = '0' then
                x_z1  <= x_in;
                y_z1  <= (others => '0');
                y_out <= x_in;

            else

                -- x[n] - x[n-1]
                diff := resize(x_in, 33) - resize(x_z1, 33);

                -- y[n-1] + diff
                sum := resize(y_z1, 33) + diff;

                -- alpha = 0.5 → divide by 2
                ytmp := shift_right(sum, 1);

                -- register updates
                x_z1  <= x_in;
                y_z1  <= ytmp(31 downto 0);
                y_out <= ytmp(31 downto 0);

            end if;

        end if;
    end process;

end rtl;

library ieee; 
use ieee.std_logic_1164.all;

entity lfsr32 is 
	port (
		clock: in std_logic;
		dout: out std_logic_vector(31 downto 0)
		);
end lfsr32;

architecture Behavioral of lfsr32 is  
	signal d: std_logic_vector(31 downto 0) := (0 => '1', others => '0');
	signal reset: std_logic;
	signal e: std_logic;
begin
	dout <= d;
	e <= d(31) xnor d(21) xnor d(1) xnor d(0);

	process (all)
	begin
		if d = x"00000000" then
			reset <= '1';
		else
			reset <= '0';
		end if;
	end process;

	process (clock)
	begin
		if rising_edge(clock) then
			d <= d(30 downto 0) & (reset or e);
		end if;
	end process;
end Behavioral;

library ieee; 
use ieee.std_logic_1164.all;

entity lfsr8 is 
	port (
		clock: in std_logic;
		dout: out std_logic_vector(7 downto 0)
		);
end lfsr8;

architecture Behavioral of lfsr8 is  
	signal d: std_logic_vector(7 downto 0) := (0 => '1', others => '0');
	signal reset: std_logic;
	signal e: std_logic;
begin
	dout <= d;
	e <= d(7) xnor d(5) xnor d(4) xnor d(3);

	process (all)
	begin
		if d = x"00" then
			reset <= '1';
		else
			reset <= '0';
		end if;
	end process;

	process (clock)
	begin
		if rising_edge(clock) then
			d <= d(6 downto 0) & (reset or e);
		end if;
	end process;
end Behavioral;