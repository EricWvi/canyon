use alloc::vec::Vec;
use core::fmt::{Debug, Formatter};
use spin::Mutex;
use x86_64::structures::paging::PhysFrame;

static mut FRAME: Option<Mutex<FrameAllocator>> = None;

pub unsafe fn init_frame(vec: Vec<MemoryRange>) {
    FRAME = Some(Mutex::new(FrameAllocator::new(vec)));
}

pub fn alloc() -> Option<PhysFrame> {
    unsafe {
        let mut fa = FRAME.as_ref().unwrap().lock();
        fa.allocate()
    }
}

pub fn alloc_range(pages: u64) -> Option<PhysFrame> {
    unsafe {
        let mut fa = FRAME.as_ref().unwrap().lock();
        fa.alloc_range(pages)
    }
}

pub fn dealloc(addr: PhysFrame) {
    unsafe {
        let mut fa = FRAME.as_ref().unwrap().lock();
        fa.deallocate(addr)
    }
}

#[derive(Eq, PartialEq)]
pub struct MemoryRange {
    pub start: PhysFrame,
    pub pages: u64,
}

impl MemoryRange {
    fn end_frame(&self) -> PhysFrame {
        self.start + self.pages - 1
    }
}

impl Debug for MemoryRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "[{:#x},{:#x}] {}kb",
            self.start.start_address().as_u64(),
            self.end_frame().start_address().as_u64() + 0xfffu64,
            self.pages * 4
        ))
    }
}

pub struct Frame(PhysFrame);

impl Drop for Frame {
    fn drop(&mut self) {
        dealloc(self.0);
    }
}

// TODO bitmap allocator
#[derive(Debug)]
pub struct FrameAllocator {
    segments: Vec<MemoryRange>,
}

impl FrameAllocator {
    pub fn new(vec: Vec<MemoryRange>) -> FrameAllocator {
        FrameAllocator { segments: vec }
    }

    fn allocate(&mut self) -> Option<PhysFrame> {
        if self.segments.len() == 0 {
            return None;
        }
        let addr = self.segments[0].start;
        if self.segments[0].pages == 1 {
            self.segments.remove(0);
        } else {
            self.segments[0].start += 1;
            self.segments[0].pages -= 1;
        }
        return Some(addr);
    }

    fn allocate_frame(&mut self) -> Option<Frame> {
        self.allocate()
            .and_then(|phys_frame| Some(Frame(phys_frame)))
    }

    fn alloc_range(&mut self, pages: u64) -> Option<PhysFrame> {
        let idx = self
            .segments
            .iter()
            .enumerate()
            .find(|(_, seg)| seg.pages >= pages);
        return if idx.is_none() {
            None
        } else {
            let id = idx.unwrap().0;
            let addr = self.segments[id].start;
            if self.segments[id].pages == pages {
                self.segments.remove(id);
            } else {
                self.segments[id].start += pages;
                self.segments[id].pages -= pages;
            }
            Some(addr)
        };
    }

    fn deallocate(&mut self, addr: PhysFrame) {
        if self.segments.len() == 0 {
            self.segments.push(MemoryRange {
                start: addr,
                pages: 1,
            });
            return;
        }
        let not = 0;
        let left = 1;
        let right = 2;
        let mut ok = not;
        let mut index = -1;
        for (i, segment) in self.segments.iter_mut().enumerate() {
            if segment.end_frame() + 1 < addr {
                continue;
            }
            if segment.end_frame() + 1 == addr {
                index = i as i32;
                ok = right;
                segment.pages += 1;
                break;
            }
            if segment.start == addr + 1 {
                index = i as i32;
                ok = left;
                segment.start = addr;
                segment.pages += 1;
                break;
            }
            if segment.start > addr + 1 {
                index = i as i32;
                break;
            }
        }
        if ok == not && index != -1 {
            self.segments.insert(
                index as usize,
                MemoryRange {
                    start: addr,
                    pages: 1,
                },
            );
            return;
        }
        if ok == not && index == -1 {
            self.segments.push(MemoryRange {
                start: addr,
                pages: 1,
            });
            return;
        }
        // ok
        if ok == left && index != 0 {
            if self.segments[(index - 1) as usize].end_frame() + 1
                == self.segments[index as usize].start
            {
                self.segments[(index - 1) as usize].pages += self.segments[index as usize].pages;
                self.segments.remove(index as usize);
            }
        }
        if ok == right && (index as usize) != self.segments.len() - 1 {
            if self.segments[index as usize].end_frame() + 1
                == self.segments[(index + 1) as usize].start
            {
                self.segments[index as usize].pages += self.segments[(index + 1) as usize].pages;
                self.segments.remove((index + 1) as usize);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use alloc::vec;
    use x86_64::structures::paging::{PhysFrame, Size4KiB};
    use x86_64::PhysAddr;

    #[test_case]
    fn case_1() {
        let mut fa = FrameAllocator {
            segments: vec![
                MemoryRange {
                    start: PhysFrame::containing_address(PhysAddr::new(1 << 12)),
                    pages: 10,
                },
                MemoryRange {
                    start: PhysFrame::containing_address(PhysAddr::new(200 << 12)),
                    pages: 1,
                },
                MemoryRange {
                    start: PhysFrame::containing_address(PhysAddr::new(203 << 12)),
                    pages: 218,
                },
                MemoryRange {
                    start: PhysFrame::containing_address(PhysAddr::new(500 << 12)),
                    pages: 101,
                },
                MemoryRange {
                    start: PhysFrame::containing_address(PhysAddr::new(602 << 12)),
                    pages: 1,
                },
                MemoryRange {
                    start: PhysFrame::containing_address(PhysAddr::new(1000 << 12)),
                    pages: 31,
                },
            ],
        };

        fa.allocate();
        assert_eq!(
            fa.segments[0],
            MemoryRange {
                start: PhysFrame::containing_address(PhysAddr::new(2 << 12)),
                pages: 9,
            }
        );

        fa.deallocate(PhysFrame::containing_address(PhysAddr::new(11 << 12)));
        assert_eq!(
            fa.segments[0],
            MemoryRange {
                start: PhysFrame::containing_address(PhysAddr::new(2 << 12)),
                pages: 10,
            }
        );

        fa.deallocate(PhysFrame::containing_address(PhysAddr::new(1 << 12)));
        assert_eq!(
            fa.segments[0],
            MemoryRange {
                start: PhysFrame::containing_address(PhysAddr::new(1 << 12)),
                pages: 11,
            }
        );

        fa.deallocate(PhysFrame::containing_address(PhysAddr::new(0)));
        assert_eq!(
            fa.segments[0],
            MemoryRange {
                start: PhysFrame::containing_address(PhysAddr::new(0)),
                pages: 12,
            }
        );
        fa.allocate();
        fa.allocate();
        fa.deallocate(PhysFrame::containing_address(PhysAddr::new(0)));
        assert_eq!(
            fa.segments[0],
            MemoryRange {
                start: PhysFrame::containing_address(PhysAddr::new(0)),
                pages: 1,
            }
        );
        assert_eq!(
            fa.segments[1],
            MemoryRange {
                start: PhysFrame::containing_address(PhysAddr::new(2 << 12)),
                pages: 10,
            }
        );
        fa.allocate();

        fa.deallocate(PhysFrame::containing_address(PhysAddr::new(199 << 12)));
        assert_eq!(
            fa.segments[1],
            MemoryRange {
                start: PhysFrame::containing_address(PhysAddr::new(199 << 12)),
                pages: 2,
            }
        );

        fa.segments[1].start = PhysFrame::containing_address(PhysAddr::new(200 << 12));
        fa.segments[1].pages -= 1;
        fa.deallocate(PhysFrame::containing_address(PhysAddr::new(201 << 12)));
        assert_eq!(
            fa.segments[1],
            MemoryRange {
                start: PhysFrame::containing_address(PhysAddr::new(200 << 12)),
                pages: 2,
            }
        );

        fa.deallocate(PhysFrame::containing_address(PhysAddr::new(202 << 12)));
        assert_eq!(
            fa.segments[1],
            MemoryRange {
                start: PhysFrame::containing_address(PhysAddr::new(200 << 12)),
                pages: 221,
            }
        );
        assert_eq!(
            fa.segments[2],
            MemoryRange {
                start: PhysFrame::containing_address(PhysAddr::new(500 << 12)),
                pages: 101,
            }
        );
        fa.deallocate(PhysFrame::containing_address(PhysAddr::new(450 << 12)));
        fa.deallocate(PhysFrame::containing_address(PhysAddr::new(601 << 12)));
        assert_eq!(
            fa.segments[3],
            MemoryRange {
                start: PhysFrame::containing_address(PhysAddr::new(500 << 12)),
                pages: 103,
            }
        );
        assert_eq!(
            fa.segments[4],
            MemoryRange {
                start: PhysFrame::containing_address(PhysAddr::new(1000 << 12)),
                pages: 31,
            }
        );

        fa.deallocate(PhysFrame::containing_address(PhysAddr::new(2000 << 12)));
        assert_eq!(
            fa.segments[fa.segments.len() - 1],
            MemoryRange {
                start: PhysFrame::containing_address(PhysAddr::new(2000 << 12)),
                pages: 1,
            }
        );
    }
}
