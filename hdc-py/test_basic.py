"""
简单的测试脚本，用于验证 hdc-rs Python 绑定是否工作正常
"""

import sys
import hdc_rs


def test_import():
    """测试模块导入"""
    try:
        import hdc_rs
        print("✓ 模块导入成功")
        return True
    except ImportError as e:
        print(f"✗ 模块导入失败: {e}")
        return False


def test_client_creation():
    """测试客户端创建"""
    try:
        from hdc_rs import HdcClient
        # 注意：这可能会失败如果 HDC 服务器未运行
        client = HdcClient("127.0.0.1:8710")
        print("✓ 客户端创建成功")
        return True
    except Exception as e:
        print(f"⚠ 客户端创建失败（这是正常的如果 HDC 服务器未运行）: {e}")
        return False


def test_list_targets():
    """测试列出设备"""
    try:
        from hdc_rs import HdcClient
        client = HdcClient("127.0.0.1:8710")
        devices = client.list_targets()
        print(f"✓ 找到 {len(devices)} 个设备")
        if devices:
            print(f"  设备: {devices}")
        return True
    except Exception as e:
        print(f"⚠ 列出设备失败: {e}")
        return False


def main():
    print("=" * 60)
    print("HDC-RS Python 绑定测试")
    print("=" * 60)
    
    tests = [
        ("导入测试", test_import),
        ("客户端创建", test_client_creation),
        ("列出设备", test_list_targets),
    ]
    
    results = []
    for name, test_func in tests:
        print(f"\n测试: {name}")
        print("-" * 40)
        result = test_func()
        results.append((name, result))
    
    print("\n" + "=" * 60)
    print("测试总结")
    print("=" * 60)
    
    passed = sum(1 for _, result in results if result)
    total = len(results)
    
    for name, result in results:
        status = "✓ 通过" if result else "✗ 失败"
        print(f"{name}: {status}")
    
    print(f"\n总计: {passed}/{total} 测试通过")
    
    if passed == total:
        print("\n🎉 所有测试通过！")
        return 0
    elif passed > 0:
        print("\n⚠ 部分测试通过（HDC 服务器未运行可能导致部分测试失败）")
        return 0
    else:
        print("\n❌ 所有测试失败")
        return 1


if __name__ == "__main__":
    sys.exit(main())
