# Fix the borrow checker error in simd_hot_path.rs

with open('/home/ubuntu/polymarket-hft-engine/src/simd_hot_path.rs', 'r') as f:
    content = f.read()

# Fix the update_side function - make it a free function
old_func = '''    #[inline(always)]
    fn update_side(
        side: &mut Vec<(PriceInt, PriceInt)>,
        price: PriceInt,
        size: PriceInt,
    ) {
        // Find position
        for i in 0..side.len() {
            if side[i].0 == price {
                if size == 0 {
                    side.remove(i);
                } else {
                    side[i].1 = size;
                }
                return;
            }
        }
        
        // New price level
        if size > 0 {
            side.push((price, size));
        }
    }'''

new_func = '''    /// Update side (free function for borrow checker)
    fn update_side_vec(
        side: &mut Vec<(PriceInt, PriceInt)>,
        price: PriceInt,
        size: PriceInt,
    ) {
        // Find position
        for i in 0..side.len() {
            if side[i].0 == price {
                if size == 0 {
                    side.remove(i);
                } else {
                    side[i].1 = size;
                }
                return;
            }
        }
        
        // New price level
        if size > 0 {
            side.push((price, size));
        }
    }'''

content = content.replace(old_func, new_func)

# Fix the calls
old_call1 = '''                    self.update_side(&mut self.bids, price_fixed, size_fixed);'''
new_call1 = '''                    Self::update_side_vec(&mut self.bids, price_fixed, size_fixed);'''

old_call2 = '''                    self.update_side(&mut self.asks, price_fixed, size_fixed);'''
new_call2 = '''                    Self::update_side_vec(&mut self.asks, price_fixed, size_fixed);'''

content = content.replace(old_call1, new_call1)
content = content.replace(old_call2, new_call2)

with open('/home/ubuntu/polymarket-hft-engine/src/simd_hot_path.rs', 'w') as f:
    f.write(content)

print('Fixed simd_hot_path.rs borrow checker')