#[cfg(feature = "nova")]
use crate::basic_block::BasicBlock;
#[cfg(feature = "nova")]
use std::collections::VecDeque;

#[cfg(feature = "nova")]
#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]
#[must_use]
pub fn get_successors(block: &BasicBlock) -> Vec<usize> {
    if let Some((last_pc, last_instr)) = block.instructions.last() {
        last_instr.control_flow_targets(*last_pc, Some(block.end_pc))
    } else {
        Vec::new()
    }
}

#[cfg(feature = "nova")]
#[must_use]
pub fn find_dead_blocks(blocks: &[BasicBlock], entry_pc: usize) -> Vec<usize> {
    if blocks.is_empty() {
        return Vec::new();
    }

    let Ok(entry_idx) = blocks.binary_search_by_key(&entry_pc, |b| b.start_pc) else {
        return blocks.iter().map(|b| b.start_pc).collect();
    };

    let mut visited = vec![false; blocks.len()];
    let mut queue = VecDeque::with_capacity(blocks.len());

    queue.push_back(entry_idx);
    visited[entry_idx] = true;

    while let Some(current_idx) = queue.pop_front() {
        let block = &blocks[current_idx];
        let successors = get_successors(block);
        for next_pc in successors {
            if let Ok(next_idx) = blocks.binary_search_by_key(&next_pc, |b| b.start_pc) {
                if !visited[next_idx] {
                    visited[next_idx] = true;
                    queue.push_back(next_idx);
                }
            }
        }
    }

    blocks
        .iter()
        .enumerate()
        .filter(|(idx, _)| !visited[*idx])
        .map(|(_, b)| b.start_pc)
        .collect()
}

#[cfg(feature = "nova")]
#[must_use]
pub fn find_shortest_path(
    blocks: &[BasicBlock],
    start_pc: usize,
    target_pc: usize,
) -> Option<Vec<usize>> {
    if blocks.is_empty() {
        return None;
    }

    let Ok(start_idx) = blocks.binary_search_by_key(&start_pc, |b| b.start_pc) else {
        return None;
    };
    let Ok(target_idx) = blocks.binary_search_by_key(&target_pc, |b| b.start_pc) else {
        return None;
    };

    let mut visited = vec![false; blocks.len()];
    let mut queue = VecDeque::with_capacity(blocks.len());
    let mut parents = vec![usize::MAX; blocks.len()];

    queue.push_back(start_idx);
    visited[start_idx] = true;

    let mut found = false;

    while let Some(current_idx) = queue.pop_front() {
        if current_idx == target_idx {
            found = true;
            break;
        }

        let block = &blocks[current_idx];
        for next_pc in get_successors(block) {
            if let Ok(next_idx) = blocks.binary_search_by_key(&next_pc, |b| b.start_pc) {
                if !visited[next_idx] {
                    visited[next_idx] = true;
                    parents[next_idx] = current_idx;
                    queue.push_back(next_idx);
                }
            }
        }
    }

    if found {
        let mut path = Vec::new();
        let mut curr = target_idx;
        while curr != start_idx {
            path.push(blocks[curr].start_pc);
            curr = parents[curr];
        }
        path.push(blocks[start_idx].start_pc);
        path.reverse();
        Some(path)
    } else {
        None
    }
}
