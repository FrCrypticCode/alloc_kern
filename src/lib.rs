#![no_std]

use core::ops::Deref;

#[derive(Clone,Copy)]
struct IdEntry{
    addr:VirtualAddr,
    frame:PhysFrame
}

#[derive(Clone,Copy)]
pub struct VirtualAddr{
    id:u16,
    pos:u32,
}
impl VirtualAddr{
    pub fn new(id:u16,pos:u32)->Self{
        VirtualAddr {id, pos}
    }
    pub fn get_pid(&self)->u16{self.id}
    pub fn get_pos(&self)->u32{self.pos}
}

#[derive(Clone,Copy)]
struct PhysFrame{
    frame:[IdFrame;2],
}
impl Deref for PhysFrame{
    type Target = IdFrame;
    fn deref(&self) -> &Self::Target {
        &self.frame[0]
    }
}
impl PhysFrame{
    fn get_size(&self)->usize{
        self.frame[1]
    }
}

#[derive(Debug)]
pub enum AllocResult{
    AllocSuccess,
    AllocPartial(usize),
    AlreadyAlloc,
    NotEnoughMemory
}

#[derive(Debug)]
pub enum DesallocResult{
    DesallocSuccess,
    AlreadyFree,
    MemoryLeak,
}
type IdFrame = usize;   
pub struct Allocator<const N:usize,const S:usize,const F:usize>{
    bytes:[u8;N],
    ids:[Option<IdEntry>;S],
    bitmap:[u8;F],
    anoms:([Option<PhysFrame>;F],usize),
    offset:usize,
}
impl<const N:usize, const S:usize,const F:usize> Allocator<N,S,F>{
    // Constructor
    pub fn new()->Option<Self>{
        if N>0 && F>0 && N>F && sqrt2(N) && sqrt2(F) && N%F==0{
            return Some(Self::generate())
        }
        None
    }
    const fn generate()->Self{
        Self { bytes:[0;N],ids:[None;S],bitmap:[0;F],anoms:([None;F],0), offset: 0 }
    }
    // Size Accessors
    const fn slots(&self)->usize{return self.bitmap.len()}
    const fn frame_size(&self)->usize{return N/F}
    // Physical Allocate
    fn alloc_phys(&mut self,need:usize)->Option<PhysFrame>{
        if need == 0 || need>self.slots(){return None}
        let ptr:usize;
        if self.offset+need>self.bitmap.len(){
            match self.find_hole(need){
                Some(p)=>{ptr = p},
                None=>{return None}
            }
        }
        else{
            ptr = self.offset.clone();          
        }
        let mut rng = (ptr,ptr+need);
        match self.verify_alloc( rng){
            AllocResult::AllocSuccess=>{
                self.lock(rng);
                self.offset = core::cmp::max(self.offset,ptr+need);
                return Some(PhysFrame {frame:[ptr,need]})
            },
            AllocResult::AlreadyAlloc=>{
                if let Some(p) = self.find_hole(need){
                    rng = (p,p+need);
                    if let AllocResult::AllocSuccess = self.verify_alloc(rng){
                        self.lock(rng);
                        self.offset = core::cmp::max(self.offset,p+need);
                        return Some(PhysFrame {frame:[p,need]})
                    }
                }
                None
            },
            _=>{None}
        }  
    }
    fn find_hole(&self,need:usize)->Option<IdFrame>{
        let mut find = (false,0,0);
        for (p,i) in self.bitmap.iter().enumerate(){
            if *i==0 && !find.0{find = (true,p,find.2+1);}
            else if *i==0 && find.0{find.2+=1;}
            else{find = (false,0,0);}
            if find.0 && find.2 == need{
                return Some(find.1)
            }
        }
        None
    }
    fn verify_alloc(&self,rng:(IdFrame,IdFrame))->AllocResult{
        for f in self.bitmap[rng.0..rng.1].iter(){
            if *f == 1{
                return AllocResult::AlreadyAlloc
            }
        }
        return AllocResult::AllocSuccess
    }
    fn lock(&mut self,rng:(IdFrame,IdFrame)){for d in self.bitmap[rng.0..rng.1].iter_mut(){*d=1}}
    // Physical Desallocate
    fn free_phys(&mut self,slot:PhysFrame)->bool{
        let rng = (slot.frame[0],slot.frame[0]+slot.frame[1]);
        match self.verify_desalloc(rng){
            DesallocResult::DesallocSuccess=>{
                self.unlock(rng);
                if self.offset != 0{
                    let mut i = self.offset-1;
                    loop{
                        if self.bitmap[i]==1{self.offset=i+1;break;}
                        if self.bitmap[i]==0&&i==0{self.offset=0;break;}
                        i -=1;
                    }
                }
                true
            },
            DesallocResult::AlreadyFree=>{false},
            _=>{false} // Nothing arrive here
        }
    }
    fn verify_desalloc(&self,rng:(IdFrame,IdFrame))->DesallocResult{
        for f in self.bitmap[rng.0..rng.1].iter(){
            if *f == 0{
                return DesallocResult::AlreadyFree
            }
        }
        return DesallocResult::DesallocSuccess
    }
    fn unlock(&mut self,rng:(IdFrame,IdFrame)){
        for f in self.bitmap[rng.0..rng.1].iter_mut(){*f=0;}
        let rngbytes = (rng.0*self.frame_size(),rng.1*self.frame_size());
        for b in self.bytes[rngbytes.0..rngbytes.1].iter_mut(){*b = 0}
    }
    //
    //Virtualize Part - Not functionnaly
    pub fn alloc(&mut self,process:u16,need:usize)->(Option<VirtualAddr>,AllocResult){ // ->Result<(*mut u8,AllocResult)>
        // Estimate number of frames need
        let nb_frames:usize;
        if need%self.frame_size()!=0{nb_frames=need/self.frame_size()+1;}
        else{nb_frames=need /self.frame_size();}
        // Checking if we have enough memory
        // One Virtual Slot = One Slice of PhysFrame resumed as struct PhysFrame
        let free_frame =self.nb_phys_frame();
        if free_frame<nb_frames && self.nb_virt_free()!=0{
            return (None,AllocResult::NotEnoughMemory)
        }
        let mut attr = 0;
        let mut ptr = 0;
        loop{
            match self.find_frames(process, need){
                Some(mut rep)=>{
                    match rep.1{
                        AllocResult::AllocSuccess=>{
                            let pos = self.get_virtslot_free().unwrap();
                            self.ids[pos] = Some(rep.0);
                            return (Some(rep.0.addr),AllocResult::AllocSuccess)
                        },
                        AllocResult::AllocPartial(x)=>{
                            if let Some(pos) =self.get_virtslot_free(){
                                attr += x;
                                rep.0.addr.pos = ptr;
                                ptr = (x * self.frame_size()) as u32;
                                self.ids[pos] = Some(rep.0);
                                if attr==need{return (Some(VirtualAddr::new(process,0)),AllocResult::AllocSuccess)}
                            }
                            else{return (Some(VirtualAddr::new(process,0)),AllocResult::AllocPartial(attr))}

                        }
                        _=>{/*Nothing arrive here*/}
                    }
                },
                None=>{
                    if ptr != 0{return (Some(VirtualAddr { id: process, pos: 0 }),AllocResult::NotEnoughMemory)}
                    else{return (None,AllocResult::NotEnoughMemory)}
                }
            }
        }     
        
    }
    fn find_frames(&mut self,process:u16,size:usize)->Option<(IdEntry,AllocResult)>{
        // All frames needed are allocate at the first request => Perfect bloc allocate
        if let Some(mem) = self.alloc_phys(size){ 
            let new =IdEntry{
                addr: VirtualAddr::new(process as u16,0),
                frame:mem
            };
            return Some((new,AllocResult::AllocSuccess)) 
        }
        // Frames are splited, need to request multiples times to make a perfect virtual access
        else{
            loop{
                match self.get_part_frame(size){
                    Some(x)=>{
                        let new =IdEntry{
                            addr: VirtualAddr::new(process,0),
                            frame:x.0
                        };
                        return Some((new,AllocResult::AllocPartial(x.1)))
                    },
                    None=>{return None}
                }
            }
            
            
        }
    }
    fn get_part_frame(&mut self,size:usize)->Option<(PhysFrame,usize)>{  //
        if size == 0{return None}
        if let Some(f) = self.alloc_phys(size){return Some((f,size))}
        else{return self.get_part_frame(size/2)}
    }
    fn get_virtslot_free(&mut self)->Option<usize>{
        for (id,entry) in self.ids.iter().enumerate(){
            if entry.is_none(){return Some(id)}
        }
        return None
    }
    fn nb_virt_free(&self)->usize{
        let mut nb = 0;
        for i in self.ids.iter(){if i.is_none(){nb+=1;}}
        return nb
    }
    fn nb_phys_frame(&self)->usize{
        let mut nb = 0;
        for i in self.bitmap.iter(){if *i==0{nb+=1;}}
        return nb
    }
    pub fn desalloc(&mut self,process:u16)->DesallocResult{
        let mut ids:[Option<usize>;S] = [None;S];
        let mut max = 0;
        for (id,entry) in self.ids.iter().enumerate(){
            if entry.is_some(){
                if entry.unwrap().addr.get_pid() == process{
                    ids[max] = Some(id);
                    max += 1;
                }
            }
        }
        for id in 0..max{
            // Unload Physical and Virtual
            if self.free_phys(self.ids[id].unwrap().frame){
                self.ids[id] = None;
            }
            else{
                self.quarantine(self.ids[id].unwrap().frame);
                self.ids[id] = None;
            }
        }
        if !self.empty_quarantine(){DesallocResult::MemoryLeak}
        else{DesallocResult::DesallocSuccess}
        
    }
    fn empty_quarantine(&self)->bool{
        for i in self.anoms.0{if i.is_some(){return false}}
        true
    }
    fn quarantine(&mut self,id:PhysFrame){
        self.anoms.0[self.anoms.1] = Some(id);
        self.anoms.1 += 1;
    }
    fn force_unlock(&mut self){
        for pf in self.anoms.0{
            if let Some(slot) = pf{
                let rng = (slot.frame[0],slot.frame[0]+slot.frame[1]);
                self.unlock(rng);
            }
            else{break}
        }
    }
}
const fn sqrt2(mut size:usize)->bool{
    if size == 0{return false}
    while size%2==0{
        size /= 2;
    }
    size == 1
}

#[cfg(test)]
mod tests{
    use crate::Allocator;
    #[test]
    fn test_generate(){
        let a = Allocator::<4096,64,64>::new();
        assert_eq!(a.is_some(),true);
    }
    #[test]
    fn test_bytes(){
        let a = Allocator::<4096,64,64>::new();
        assert_eq!(a.unwrap().bytes.len(),4096)
    }
    #[test]
    fn test_virt(){
        let a = Allocator::<4096,64,64>::new();
        assert_eq!(a.unwrap().ids.len(),64)
    }
    #[test]
    fn test_frames(){
        let a = Allocator::<4096,64,64>::new();
        assert_eq!(a.unwrap().bitmap.len(),64)
    }
    #[test]
    fn test_alloc(){
        let mut a = Allocator::<4096,64,64>::new().unwrap();
        a.alloc_phys(4);
        assert_eq!(a.nb_phys_frame(),64-4);
    }
    #[test]
    fn test_out_of_memory(){
        let mut a = Allocator::<4096,64,64>::new().unwrap();
        for i in a.bitmap.iter_mut(){
            *i = 1;
        }
        assert_eq!(a.alloc_phys(1).is_none(),true)
    }
    #[test]
    fn test_free(){
        let mut a = Allocator::<4096,64,64>::new().unwrap();
        let frame = a.alloc_phys(4);
        a.free_phys(frame.unwrap());
        assert_eq!(a.nb_phys_frame(),64);
    }
    #[test]
    fn test_find_hole(){
        let mut a = Allocator::<4096,64,64>::new().unwrap();
        for i in a.bitmap.iter_mut(){
            *i = 1;
        }
        a.bitmap[2]=0;
        a.bitmap[3]=0;
        a.alloc_phys(1).unwrap();
        assert_eq!(a.bitmap[2],1)
    }
}