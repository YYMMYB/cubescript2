# 方块脚本

很可能咕咕咕. 后面那个 2 就是因为之前已经重新弄过一次了.

目前还几乎是啥都没做, 啥功能都没有.

## 目标

一个3d空间, 里面有: 方块,正方形,线段,点, 分布方式类似与MineCraft(坐标是边长的整数倍,轴对齐). 方便起见, 后面将这四种几何体称为基元.

通过对基元进行组合,与在基元上附加一些信息, 来描述其他(或自身)基元的运动,创建销毁等事件. 从而可以生成由基元构成的模型, 模型动画等.

- 多个基元的组合: 主要表示容易通过空间位置表示的信息, 例如相对位置等
- 基元上的附加信息: 主要表示不容易通过基元表示的信息, 例如数字, 颜色等

逻辑该如何表示? 还没想.

基元的大小可以是2的幂次.

提供函数的功能. 将多个基元描述的规则, 封装到几个用于表示输入输出的基元上.


## 进度

| 进度   | 功能     |
| ------ | -------- |
| 正在做 | 渲染     |
| 未开始 | 场景管理 |
| 未开始 | 语言     |

### 做了什么, 正在做什么

目前可以画三角形了.

做完了: 
- 简单的 Camera 控制
- GPU Instance 的封装

正在做:
1. 渲染一个方块(通过正方形的GPU实例渲染, 主要处理24种旋转以及镜像)
1. 加贴图
1. 渲染多个方块(主要处理多次drawcall, 目前只有一个drawcall)

后面要做:
1. 重构
1. 基元的管理(类似于MineCraft的Chunk)
1. 贴图支持(一个chunk多个贴图的处理, 一次可能无法bind那么多贴图?, chunk构建的时候就应该分drawcall构建)
  - 每种方块平均4张`64*64`的(4张是猜的), 一张贴图是`2048*2048`, 一共`2^5 * 2^5=1024`种方块, 怎么也得支持10000+种方块吧, 因此可能会用到10+张图.
  - 关键是支持mod的话, 人家想做高清材质, 每种方块的贴图可能会很大.
1. 射线检查的支持(目前主要用于鼠标点击)
1. 测试用的随机场景生成
1. 视锥体剔除