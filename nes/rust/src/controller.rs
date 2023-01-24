pub trait NesController {
    fn update_latch_bits(&mut self, data: [bool; 3]);
    fn read_data(&mut self) -> u8;
}
