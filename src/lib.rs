#![no_std]

#[derive(Clone,Copy)]
struct IdEntry{
    addr:VirtualAddr,
    frame:u16,
    ptr:*mut u8
}

#[derive(Clone,Copy)]
struct VirtualAddr{
    id:u16,
    pos:u16
}
impl VirtualAddr{
    pub fn new(id:u16,pos:u16)->Self{
        VirtualAddr { id, pos }
    }
    pub fn get_pid(&self)->u16{self.id}
    pub fn get_pos(&self)->u16{self.pos}
}

struct PhysFrame{
    frame:IdFrame,
    ptr:*mut u8
}

#[derive(Debug)]
enum AllocResult{
    AllocSuccess,
    AlreadyAlloc,
    NoMemory
}

#[derive(Debug)]
enum DesallocResult{
    DesallocSuccess,
    AlreadyFree
}
type IdFrame = usize;   // Réflexion sur le type : usize peut être trop volumineux
pub struct Allocator<const N:usize,const S:usize,const F:usize>{
    bytes:[u8;N],
    ids:[Option<IdEntry>;S],
    bitmap:[u8;F],
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
        Self { bytes:[0;N],ids:[None;S],bitmap:[0;F], offset: 0 }
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
        let mut rngbytes = (rng.0*self.frame_size(),rng.1*self.frame_size());
        match self.verify_alloc( rng){
            AllocResult::AllocSuccess=>{
                self.lock(rng);
                self.offset = core::cmp::max(self.offset,ptr+need);
                return Some(PhysFrame {frame:ptr,ptr:self.bytes[rngbytes.0..rngbytes.1].as_mut_ptr()})
                //return Some((ptr,self.bytes[rngbytes.0..rngbytes.1].as_mut_ptr()))
            },
            AllocResult::AlreadyAlloc=>{
                if let Some(p) = self.find_hole(need){
                    rng = (p,p+need);
                    rngbytes = (rng.0*self.frame_size(),rng.1*self.frame_size());
                    if let AllocResult::AllocSuccess = self.verify_alloc(rng){
                        self.lock(rng);
                        self.offset = core::cmp::max(self.offset,p+need);
                        return Some(PhysFrame {frame:ptr,ptr:self.bytes[rngbytes.0..rngbytes.1].as_mut_ptr()})
                    }
                }
                None
            },
            AllocResult::NoMemory=>{None}
        }  
    }
    fn find_hole(&self,need:usize)->Option<(IdFrame)>{
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
    fn free_phys(&mut self,slots:&mut [u8])->bool{
        let ptr = slots.as_ptr() as usize;
        let start = self.bytes.as_ptr() as usize;
        let start_frame = (ptr - start)/self.frame_size();
        let rng = (start_frame,start_frame+slots.len()/self.frame_size());
        match self.verify_desalloc(rng){
            DesallocResult::DesallocSuccess=>{
                self.unlock((rng.0,rng.1));
                if self.offset != 0{
                    let mut i = self.offset-1;
                    loop{
                        if(self.bitmap[i]==1){self.offset=i+1;break;}
                        if(self.bitmap[i]==0&&i==0){self.offset=0;break;}
                        i -=1;
                    }
                }
                true
            },
            DesallocResult::AlreadyFree=>{false}
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
    fn unlock(&mut self,rng:(IdFrame,IdFrame)){for f in self.bitmap[rng.0..rng.1].iter_mut(){*f=0;}}
    //
    //Virtualize Part - Not functionnaly
    pub fn alloc(&mut self,process:u16,need:usize){ // ->Result<(*mut u8,AllocResult)>
        let nb_frames:usize;
        if need%self.frame_size()!=0{nb_frames=need/self.frame_size()+1;}
        else{nb_frames=need /self.frame_size();}
        // All frames needed are allocate at the first request => Perfect bloc allocate
        if let Some(mem) = self.alloc_phys(nb_frames){
            let new =IdEntry{
                addr: VirtualAddr::new(mem.frame as u16,0),
                frame:mem.frame as u16,
                ptr:mem.ptr
            };
            for i in self.ids.iter_mut(){
                if i.is_none(){*i=Some(new);}
            }
        }
        else{
            // Frames are splited, need to request multiples times to make a perfect virtual access
            // get_part_frame !!!
        }
        /*
            struct IdEntry{
                id:u16,
                addr:u16,
                frame:u16,
                ptr:*mut u8
            }
        */
    }
    fn get_part_frame(&mut self,size:usize)->Option<PhysFrame>{  //
        if size == 0{
            return None
        }
        if let Some(f) = self.alloc_phys(size){
            return Some(f)
        }
        else{
            return self.get_part_frame(size/2)
        }
        
    }
    pub fn desalloc(&mut self,process:u16)->DesallocResult{
        DesallocResult::DesallocSuccess
    }
}
const fn sqrt2(mut size:usize)->bool{
    if size == 0{return false}
    while size%2==0{
        size /= 2;
    }
    size == 1
}

