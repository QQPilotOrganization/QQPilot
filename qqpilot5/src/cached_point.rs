

use anyhow::Result;


pub(crate) struct CachedPoint {
    pub(crate) point: (u32, u32),
    pub(crate) expiry: u8,
}

impl CachedPoint {

    //新建缓存失效点
    pub fn new_expired() -> Self {
        Self {
            point: (0,0),
            expiry: 0,
        }
    }
    //更新并恢复失效次数
    pub fn update(&mut self,p: (u32, u32))
    {
        self.point=p;
        self.expiry=255;
    }
    //是否失效
    pub fn expires(&self) -> bool {
        return self.expiry == 0;
    }
    //使用一次，失效-1
    pub fn consume(&mut self) -> Result<(u32, u32)> {
        if self.expires() {
            anyhow::bail!("expired")
        }
        self.expiry -= 1;
        Ok(self.point)
    }
    //该点立刻失效
    pub fn expire(&mut self)
    {
        self.expiry=0;
    }

}
