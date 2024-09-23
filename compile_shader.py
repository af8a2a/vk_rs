import os
import subprocess

def compile_shader(shader_path, output_path):
    # 调用 glslangValidator 工具编译 Shader
    try:
        subprocess.run(
            ['glslangValidator', '-V', shader_path, '-o', output_path],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE
        )
        print(f"Compiled {shader_path} -> {output_path}")
    except subprocess.CalledProcessError as e:
        print(f"Failed to compile {shader_path}: {e.stderr.decode()}")

def compile_shaders_in_directory(directory):
    # 递归遍历文件夹
    for root, _, files in os.walk(directory):
        for file in files:
            if file.endswith(('.vert', '.frag', '.comp', '.geom', '.tesc', '.tese')):  # 根据需求添加其他扩展名
                shader_path = os.path.join(root, file)
                output_path = shader_path + ".spv"  # 输出文件路径
                compile_shader(shader_path, output_path)

if __name__ == "__main__":
    shader_directory = "shader"  # 替换为你的 Shader 文件夹路径
    compile_shaders_in_directory(shader_directory)
