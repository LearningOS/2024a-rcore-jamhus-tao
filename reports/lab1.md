1. 三个 bad 示例
  1.1 `ch2b_bad_address`: [kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.
    core 库成功的抛出了 PageFault 错误, 并陷入内核态处理. trap_handler 成功抓取了这个错误.
  1.2 `ch2b_bad_instructions`: [kernel] IllegalInstruction in application, kernel killed it.
    用户空间没有执行 sret 的权限, 因为 sret 用于内核返回到用户空间. 同样地 trap_handler 成功抓取了这个错误.
  1.3 `ch2b_bad_register`: [kernel] IllegalInstruction in application, kernel killed it.
    同样地, 用户空间没有访问 sstatus 的权限.
2. 关于 trap.S
  2.1 __restore 进入时 a0 是用户空间栈指针
  2.2 sstatus 用于保存 trap 发生之前 CPU 处在哪个特权级（S/U）等信息
      sepc 用于保存 Trap 发生之前执行的最后一条指令的地址
      sscratch 用于保存指向内核态的指针
  2.3 x2 用于存储 sp, 而这里 sp 被用作特殊考虑无需转储, 之后用户存储 sscratch
      x4 用于存储 tp, 用户不会使用它因此无需转储
  2.4 L60: 实际交换了 sp 和 sscratch 的值, 完成用户空间和内核空间栈指针的交换, 此时 sp 为用户空间栈指针
  2.5 sret 指令用于完成最终的用户态向内核态切换
  2.6 L13: 实际交换了 sp 和 sscratch 的值, 完成用户空间和内核空间栈指针的交换, 此时 sp 为内核空间栈指针
  2.7 从用户态进入内核态是由 ecall 触发的



**荣誉准则**
----------------
.. warning::
    
    请把填写了《你的说明》的下述内容拷贝到的到实验报告中。
    否则，你的提交将视作无效，本次实验的成绩将按“0”分计。

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

    * 与队长 [chenhongyi](https://opencamp.cn/user/chenhongyi) 分享各章节心得，在环境配置问题上提供支持，没有代码上的交流。

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

    * 参考官方文档 https://learningos.cn/rCore-Camp-Guide-2024A
    * 另外, 参考 Kimi 提供的解释

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。
我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。
我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。
我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。
我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。

