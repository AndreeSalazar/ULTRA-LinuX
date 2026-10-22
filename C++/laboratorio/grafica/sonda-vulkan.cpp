// SPDX-License-Identifier: GPL-2.0-only
// CPU-only laboratory: validation callback aborts the invalid call before dispatch.
#include <vulkan/vulkan.h>
#include <atomic>
#include <cstdio>
#include <cstdint>
#include <cstring>
#include <vector>

namespace {
struct Evidence { std::atomic<unsigned> errors{0}; std::atomic<bool> size_vuid{false}; };
VKAPI_ATTR VkBool32 VKAPI_CALL message(VkDebugUtilsMessageSeverityFlagBitsEXT severity,
    VkDebugUtilsMessageTypeFlagsEXT, const VkDebugUtilsMessengerCallbackDataEXT* data, void* user) {
    auto& evidence = *static_cast<Evidence*>(user);
    if ((severity & VK_DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT) != 0) {
        ++evidence.errors;
        const char* id = data->pMessageIdName ? data->pMessageIdName : "sin-VUID";
        if (std::strcmp(id, "VUID-VkBufferCreateInfo-size-00912") == 0) evidence.size_vuid.store(true);
        std::printf("VALIDACION %s\n", id);
        return VK_TRUE;
    }
    return VK_FALSE;
}
}

int main() {
    std::setvbuf(stdout, nullptr, _IONBF, 0);
    Evidence evidence;
    VkDebugUtilsMessengerCreateInfoEXT debug{};
    debug.sType = VK_STRUCTURE_TYPE_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT;
    debug.messageSeverity = VK_DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT;
    debug.messageType = VK_DEBUG_UTILS_MESSAGE_TYPE_VALIDATION_BIT_EXT;
    debug.pfnUserCallback = message;
    debug.pUserData = &evidence;
    const char* layer = "VK_LAYER_KHRONOS_validation";
    const char* extension = VK_EXT_DEBUG_UTILS_EXTENSION_NAME;
    VkInstanceCreateInfo instance_info{};
    instance_info.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO;
    instance_info.pNext = &debug;
    instance_info.enabledLayerCount = 1;
    instance_info.ppEnabledLayerNames = &layer;
    instance_info.enabledExtensionCount = 1;
    instance_info.ppEnabledExtensionNames = &extension;
    VkInstance instance = VK_NULL_HANDLE;
    const VkResult instance_result = vkCreateInstance(&instance_info, nullptr, &instance);
    if (instance_result != VK_SUCCESS) {
        std::fprintf(stderr, "ROJO vkCreateInstance=%d\n", static_cast<int>(instance_result));
        return 1;
    }
    auto create_debug = reinterpret_cast<PFN_vkCreateDebugUtilsMessengerEXT>(vkGetInstanceProcAddr(instance, "vkCreateDebugUtilsMessengerEXT"));
    auto destroy_debug = reinterpret_cast<PFN_vkDestroyDebugUtilsMessengerEXT>(vkGetInstanceProcAddr(instance, "vkDestroyDebugUtilsMessengerEXT"));
    VkDebugUtilsMessengerEXT messenger = VK_NULL_HANDLE;
    if (!create_debug || !destroy_debug || create_debug(instance, &debug, nullptr, &messenger) != VK_SUCCESS) {
        vkDestroyInstance(instance, nullptr); return 1;
    }
    bool passed = false;
    VkDevice device = VK_NULL_HANDLE;
    std::uint32_t count = 0;
    if (vkEnumeratePhysicalDevices(instance, &count, nullptr) == VK_SUCCESS && count > 0) {
        std::vector<VkPhysicalDevice> physical(count);
        if (vkEnumeratePhysicalDevices(instance, &count, physical.data()) == VK_SUCCESS) {
            VkPhysicalDeviceProperties properties{};
            vkGetPhysicalDeviceProperties(physical[0], &properties);
            std::printf("VULKAN dispositivo=%s tipo=%u\n", properties.deviceName, static_cast<unsigned>(properties.deviceType));
            // Reject a real GPU even if an operator forgot to isolate the process.
            if (properties.deviceType == VK_PHYSICAL_DEVICE_TYPE_CPU) {
                std::uint32_t queues = 0;
                vkGetPhysicalDeviceQueueFamilyProperties(physical[0], &queues, nullptr);
                std::vector<VkQueueFamilyProperties> families(queues);
                vkGetPhysicalDeviceQueueFamilyProperties(physical[0], &queues, families.data());
                for (std::uint32_t index = 0; index < queues; ++index) {
                    if (families[index].queueCount == 0) continue;
                    const float priority = 1.0F;
                    VkDeviceQueueCreateInfo queue{};
                    queue.sType = VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO;
                    queue.queueFamilyIndex = index; queue.queueCount = 1; queue.pQueuePriorities = &priority;
                    VkDeviceCreateInfo info{};
                    info.sType = VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO;
                    info.queueCreateInfoCount = 1; info.pQueueCreateInfos = &queue;
                    if (vkCreateDevice(physical[0], &info, nullptr, &device) != VK_SUCCESS) break;
                    VkBufferCreateInfo buffer_info{};
                    buffer_info.sType = VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO;
                    buffer_info.size = 4096;
                    buffer_info.usage = VK_BUFFER_USAGE_TRANSFER_SRC_BIT;
                    buffer_info.sharingMode = VK_SHARING_MODE_EXCLUSIVE;
                    VkBuffer buffer = VK_NULL_HANDLE;
                    const bool valid = vkCreateBuffer(device, &buffer_info, nullptr, &buffer) == VK_SUCCESS;
                    if (valid) vkDestroyBuffer(device, buffer, nullptr);
                    const unsigned before = evidence.errors.load();
                    buffer_info.size = 0;
                    buffer = VK_NULL_HANDLE;
                    const VkResult invalid = vkCreateBuffer(device, &buffer_info, nullptr, &buffer);
                    // No memory is allocated, no queue submits or shader dispatches occur.
                    if (invalid == VK_SUCCESS) vkDestroyBuffer(device, buffer, nullptr);
                    passed = valid && before == 0 && evidence.errors.load() > before && evidence.size_vuid.load()
                        && invalid == VK_ERROR_VALIDATION_FAILED_EXT;
                    break;
                }
            }
        }
    }
    if (device != VK_NULL_HANDLE) vkDestroyDevice(device, nullptr);
    destroy_debug(instance, messenger, nullptr);
    vkDestroyInstance(instance, nullptr);
    std::puts(passed ? "VERDE: buffer valido y llamada invalida bloqueada por validacion" : "ROJO: resultado Vulkan inesperado");
    return passed ? 0 : 1;
}
